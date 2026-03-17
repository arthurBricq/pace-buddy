use chrono::Utc;
use domain::{
    DomainError, RunningCoachMemory, RunningCoachMemoryData, RunningCoachMessage,
    RunningCoachSettings, RunningCoachState,
};
use llm::{ChatMessage, LlmClient};
use uuid::Uuid;

use crate::{
    build_coach_context, update_memory_after_exchange, CoachContextBundle, CoachMemoryDataStore,
};

pub struct CoachMemory<S: CoachMemoryDataStore> {
    store: S,
}

const COACH_SYSTEM_PROMPT: &str = "You are a persistent running coach.
You are opinionated, practical, and continuity-focused.
Always ground advice in the provided context and memory snapshot.
Use metric units and specific guidance for the next sessions.
The context contains a section 'Recent user prompts' with only user messages (no coach replies). Use it to avoid repeating prior guidance.
Do not repeat all context; focus on decisions and coaching direction.";

impl<S: CoachMemoryDataStore> CoachMemory<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn build_context(
        &self,
        user_id: Uuid,
        settings: &RunningCoachSettings,
        state: &RunningCoachState,
        memory: &RunningCoachMemoryData,
    ) -> Result<CoachContextBundle, DomainError> {
        build_coach_context(&self.store, user_id, settings, state, memory).await
    }

    pub(crate) async fn update_memory_after_exchange(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        settings: &RunningCoachSettings,
        user_message: &RunningCoachMessage,
        assistant_message: &RunningCoachMessage,
        memory: &mut RunningCoachMemory,
    ) {
        update_memory_after_exchange(
            llm_client,
            settings,
            user_message,
            assistant_message,
            memory,
        )
        .await;
    }

    pub async fn send_message(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        user_id: Uuid,
        user_content: &str,
    ) -> Result<RunningCoachMessage, DomainError> {
        log::info!(
            "coach.send_message start user_id={} content_len={}",
            user_id,
            user_content.len()
        );

        // Load the previous state

        let settings = self
            .store
            .get_or_create_running_coach_settings(user_id)
            .await?;
        let mut memory = self
            .store
            .get_or_create_running_coach_memory(user_id)
            .await?;
        let mut coach_state = self
            .store
            .get_or_create_running_coach_state(user_id)
            .await?;
        log::info!(
            "coach.send_message loaded_state user_id={} model={} mem_pinned={} mem_episodic={} mem_norm_count={} last_seen_activity={}",
            user_id,
            settings.model,
            memory.data.pinned_facts.len(),
            memory.data.episodic_memory.len(),
            memory.message_count_since_normalization,
            coach_state
                .last_seen_activity_start_date
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "none".to_string())
        );

        // Create and store user message

        let user_message = RunningCoachMessage {
            id: Uuid::new_v4(),
            user_id,
            role: "user".to_string(),
            content: user_content.to_string(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cost: 0.0,
            created_at: Utc::now(),
        };
        self.store
            .store_running_coach_message(&user_message)
            .await?;
        log::info!(
            "coach.send_message user_message_stored user_id={} message_id={}",
            user_id,
            user_message.id
        );

        // Build coach context

        let context_bundle = self
            .build_context(user_id, &settings, &coach_state, &memory.data)
            .await?;
        log::info!(
            "coach.send_message context_built user_id={} context_chars={} new_last_seen_activity={}",
            user_id,
            context_bundle.content.len(),
            context_bundle
                .latest_seen_activity_start_date
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "none".to_string())
        );

        //

        let mut llm_messages = vec![
            ChatMessage::system(COACH_SYSTEM_PROMPT),
            ChatMessage::system(format!("Coach personality: {}", settings.personality)),
            ChatMessage::system(context_bundle.content),
        ];

        let history = self.store.list_running_coach_messages(user_id, 24).await?;
        log::info!(
            "coach.send_message history_loaded user_id={} history_messages={}",
            user_id,
            history.len()
        );
        for msg in history {
            llm_messages.push(ChatMessage {
                role: msg.role,
                content: msg.content,
            });
        }

        log::info!(
            "coach.send_message llm_call_start user_id={} model={} llm_messages={}",
            user_id,
            settings.model,
            llm_messages.len()
        );
        let result = llm_client
            .chat_completion(&settings.model, llm_messages, None)
            .await
            .map_err(|e| DomainError::Internal(format!("LLM call failed: {e}")))?;
        log::info!(
            "coach.send_message llm_call_done user_id={} prompt_tokens={} completion_tokens={} total_tokens={} real_cost={:.6}",
            user_id,
            result.usage.prompt_tokens,
            result.usage.completion_tokens,
            result.usage.total_tokens,
            result.usage.cost
        );

        let assistant_message = RunningCoachMessage {
            id: Uuid::new_v4(),
            user_id,
            role: "assistant".to_string(),
            content: result.content,
            prompt_tokens: result.usage.prompt_tokens,
            completion_tokens: result.usage.completion_tokens,
            total_tokens: result.usage.total_tokens,
            cost: result.usage.cost,
            created_at: Utc::now(),
        };
        self.store
            .store_running_coach_message(&assistant_message)
            .await?;
        log::info!(
            "coach.send_message assistant_message_stored user_id={} message_id={}",
            user_id,
            assistant_message.id
        );

        coach_state.last_interaction_at = Some(Utc::now());
        coach_state.last_seen_activity_start_date = context_bundle.latest_seen_activity_start_date;
        coach_state.updated_at = Utc::now();
        self.store.upsert_running_coach_state(&coach_state).await?;
        log::info!(
            "coach.send_message state_updated user_id={} last_interaction_at={} last_seen_activity={}",
            user_id,
            coach_state
                .last_interaction_at
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "none".to_string()),
            coach_state
                .last_seen_activity_start_date
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "none".to_string())
        );

        self.update_memory_after_exchange(
            llm_client,
            &settings,
            &user_message,
            &assistant_message,
            &mut memory,
        )
        .await;
        self.store.upsert_running_coach_memory(&memory).await?;
        log::info!(
            "coach.send_message memory_saved user_id={} mem_pinned={} mem_episodic={} mem_norm_count={}",
            user_id,
            memory.data.pinned_facts.len(),
            memory.data.episodic_memory.len(),
            memory.message_count_since_normalization
        );
        log::info!("coach.send_message done user_id={}", user_id);

        Ok(assistant_message)
    }
}
