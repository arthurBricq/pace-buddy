use async_trait::async_trait;
use chrono::Utc;
use domain::{
    DomainError, RunningCoachMemory, RunningCoachMemoryData, RunningCoachMessage,
    RunningCoachSettings, RunningCoachState,
};
use llm::{ChatMessage, LlmClient, LlmUsage, ToolCall, ToolChoice, ToolDefinition};
use uuid::Uuid;

use crate::{
    build_coach_context, update_memory_after_exchange, CoachContextBundle, CoachMemoryDataStore,
};

pub struct CoachMemory<S: CoachMemoryDataStore> {
    store: S,
}

#[async_trait]
pub trait CoachToolExecutor: Send + Sync {
    fn tool_definitions(&self) -> Vec<ToolDefinition>;
    async fn execute_tool_call(
        &self,
        user_id: Uuid,
        call: &ToolCall,
    ) -> Result<String, DomainError>;
    fn summarize_tool_result(&self, _call: &ToolCall, _tool_output: &str) -> Option<String> {
        None
    }
}

const COACH_SYSTEM_PROMPT: &str = "You are a persistent running coach.
You are opinionated, practical, and continuity-focused.
Always ground advice in the provided context and memory snapshot.
Use metric units and specific guidance for the next sessions.
The context contains a section 'Recent user prompts' with only user messages (no coach replies). Use it to avoid repeating prior guidance.
Do not repeat all context; focus on decisions and coaching direction.";

const COACH_TOOL_PROMPT: &str = "You have access to session lookup tools.
Use tools instead of guessing whenever the answer depends on one or more specific sessions.
Use get_last_sessions for requests about the latest or most recent session(s).
Use get_sessions_in_time_range for requests about a date or date range.
Use search_sessions for fuzzy name/tag/text matching.
Respect the coach context note describing whether TrailRun is treated as a run.
Never invent an activity_id.
If multiple plausible matches are returned, ask the user to choose one session before using get_session_detail.
Only use get_session_detail once an activity_id is unambiguous.";

const MAX_TOOL_LOOP_STEPS: usize = 4;
const MAX_RECENT_TOOL_RESULTS: usize = 6;
const MAX_COACH_HISTORY_MESSAGES: i64 = 20;

struct CoachReplyOutcome {
    content: String,
    usage: LlmUsage,
    recent_tool_results: Vec<String>,
}

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
        self.send_message_internal(llm_client, user_id, user_content, None)
            .await
    }

    pub async fn send_message_with_tools(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        user_id: Uuid,
        user_content: &str,
        tool_executor: &(dyn CoachToolExecutor + Send + Sync),
    ) -> Result<RunningCoachMessage, DomainError> {
        self.send_message_internal(llm_client, user_id, user_content, Some(tool_executor))
            .await
    }

    async fn send_message_internal(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        user_id: Uuid,
        user_content: &str,
        tool_executor: Option<&(dyn CoachToolExecutor + Send + Sync)>,
    ) -> Result<RunningCoachMessage, DomainError> {
        log::info!(
            "coach.send_message start user_id={} content_len={} tools_enabled={}",
            user_id,
            user_content.len(),
            tool_executor.is_some()
        );

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

        let mut llm_messages = vec![
            ChatMessage::system(COACH_SYSTEM_PROMPT),
            ChatMessage::system(format!("Coach personality: {}", settings.personality)),
            ChatMessage::system(context_bundle.content),
        ];
        if tool_executor.is_some() {
            llm_messages.push(ChatMessage::system(COACH_TOOL_PROMPT));
        }

        let history = self.store.list_running_coach_messages(user_id, MAX_COACH_HISTORY_MESSAGES).await?;
        log::info!(
            "coach.send_message history_loaded user_id={} history_messages={}",
            user_id,
            history.len()
        );
        for msg in history {
            if msg.role != "user" && msg.role != "assistant" {
                log::warn!(
                    "coach.send_message ignoring unexpected history role user_id={} role={} message_id={}",
                    user_id,
                    msg.role,
                    msg.id
                );
                continue;
            }
            llm_messages.push(ChatMessage::new(msg.role, msg.content));
        }

        let result = self
            .complete_coach_reply(
                llm_client,
                &settings.model,
                llm_messages,
                user_id,
                tool_executor,
            )
            .await?;
        merge_recent_tool_results(
            &mut memory.data.recent_tool_results,
            result.recent_tool_results,
        );
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

    async fn complete_coach_reply(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        model: &str,
        mut llm_messages: Vec<ChatMessage>,
        user_id: Uuid,
        tool_executor: Option<&(dyn CoachToolExecutor + Send + Sync)>,
    ) -> Result<CoachReplyOutcome, DomainError> {
        if let Some(executor) = tool_executor {
            let tools = executor.tool_definitions();
            if !tools.is_empty() {
                return self
                    .complete_with_tools(
                        llm_client,
                        model,
                        &mut llm_messages,
                        user_id,
                        executor,
                        tools,
                    )
                    .await;
            }
        }

        log::info!(
            "coach.send_message llm_call_start model={} llm_messages={}",
            model,
            llm_messages.len()
        );

        let result = llm_client
            .chat_completion(model, llm_messages, None)
            .await
            .map_err(|e| DomainError::Internal(format!("LLM call failed: {e}")))?;
        Ok(CoachReplyOutcome {
            content: result.content,
            usage: result.usage,
            recent_tool_results: Vec::new(),
        })
    }

    async fn complete_with_tools(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        model: &str,
        llm_messages: &mut Vec<ChatMessage>,
        user_id: Uuid,
        executor: &(dyn CoachToolExecutor + Send + Sync),
        tools: Vec<ToolDefinition>,
    ) -> Result<CoachReplyOutcome, DomainError> {
        let mut usage_total = LlmUsage::default();
        let mut recent_tool_results = Vec::new();

        for step in 0..MAX_TOOL_LOOP_STEPS {
            log::info!(
                "coach.send_message tool_loop_step={} model={} llm_messages={}",
                step + 1,
                model,
                llm_messages.len()
            );

            let completion = llm_client
                .chat_completion_with_tools(
                    model,
                    llm_messages.clone(),
                    tools.clone(),
                    Some(ToolChoice::Auto),
                    Some(false),
                    None,
                )
                .await
                .map_err(|e| DomainError::Internal(format!("LLM tool-call failed: {e}")))?;

            accumulate_usage(&mut usage_total, &completion.usage);

            if completion.tool_calls.is_empty() {
                let content = completion.content.unwrap_or_default().trim().to_string();
                if content.is_empty() {
                    return Err(DomainError::Internal(
                        "LLM returned empty final assistant content in tool loop".to_string(),
                    ));
                }

                return Ok(CoachReplyOutcome {
                    content,
                    usage: usage_total,
                    recent_tool_results,
                });
            }

            llm_messages.push(ChatMessage::assistant_tool_calls(&completion.tool_calls));

            for call in completion.tool_calls {
                let tool_output = match executor.execute_tool_call(user_id, &call).await {
                    Ok(output) => output,
                    Err(err) => serde_json::json!({
                        "error": err.to_string(),
                        "tool": call.name,
                        "tool_call_id": call.id,
                    })
                    .to_string(),
                };
                if let Some(summary) = executor.summarize_tool_result(&call, &tool_output) {
                    push_recent_tool_result(&mut recent_tool_results, summary);
                }

                llm_messages.push(ChatMessage::tool(call.id.clone(), tool_output));
            }
        }

        Err(DomainError::Internal(format!(
            "Coach tool loop exceeded {} steps without final assistant message",
            MAX_TOOL_LOOP_STEPS
        )))
    }
}

fn accumulate_usage(target: &mut LlmUsage, incoming: &LlmUsage) {
    target.prompt_tokens = target.prompt_tokens.saturating_add(incoming.prompt_tokens);
    target.completion_tokens = target
        .completion_tokens
        .saturating_add(incoming.completion_tokens);
    target.total_tokens = target.total_tokens.saturating_add(incoming.total_tokens);
    target.cost += incoming.cost;
}

fn merge_recent_tool_results(target: &mut Vec<String>, additions: Vec<String>) {
    for addition in additions {
        push_recent_tool_result(target, addition);
    }
}

fn push_recent_tool_result(target: &mut Vec<String>, summary: String) {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        return;
    }
    if target.iter().any(|existing| existing == trimmed) {
        return;
    }
    target.push(trimmed.to_string());
    if target.len() > MAX_RECENT_TOOL_RESULTS {
        let start = target.len() - MAX_RECENT_TOOL_RESULTS;
        *target = target[start..].to_vec();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use domain::{
        Activity, AthleteProfile, DomainError, IdentityProfile, RunningCoachMemory,
        RunningCoachMessage, RunningCoachSettings, RunningCoachState, User,
    };
    use llm::{
        ChatCompletionResult, ChatMessage, JsonSchemaDefinition, LlmClient, LlmError, LlmUsage,
        ModelInfo, ToolCall, ToolChoice, ToolCompletionResult, ToolDefinition,
    };
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use crate::{CoachMemory, CoachMemoryDataStore, CoachToolExecutor};

    struct FakeStore {
        user: User,
        messages: Arc<Mutex<Vec<RunningCoachMessage>>>,
        settings: RunningCoachSettings,
        memory: Arc<Mutex<RunningCoachMemory>>,
        state: RunningCoachState,
    }

    impl FakeStore {
        fn new(user_id: Uuid) -> Self {
            let user = User {
                id: user_id,
                username: "runner".to_string(),
                display_name: "Runner".to_string(),
                email: None,
                created_at: Utc::now(),
                mas_current: Some(16.0),
                quota_balance_usd: 10.0,
            };

            let mut settings = RunningCoachSettings::default();
            settings.user_id = user_id;
            settings.normalizer_every_n_messages = 50;

            let mut memory = RunningCoachMemory::default();
            memory.user_id = user_id;

            let state = RunningCoachState {
                user_id,
                last_interaction_at: None,
                last_seen_activity_start_date: None,
                updated_at: Utc::now(),
            };

            Self {
                user,
                messages: Arc::new(Mutex::new(Vec::new())),
                settings,
                memory: Arc::new(Mutex::new(memory)),
                state,
            }
        }

        async fn saved_memory(&self) -> RunningCoachMemory {
            self.memory.lock().await.clone()
        }
    }

    #[async_trait]
    impl CoachMemoryDataStore for FakeStore {
        async fn get_or_create_running_coach_settings(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachSettings, DomainError> {
            Ok(self.settings.clone())
        }

        async fn get_or_create_running_coach_memory(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachMemory, DomainError> {
            Ok(self.memory.lock().await.clone())
        }

        async fn get_or_create_running_coach_state(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachState, DomainError> {
            Ok(self.state.clone())
        }

        async fn upsert_running_coach_state(
            &self,
            _state: &RunningCoachState,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn upsert_running_coach_memory(
            &self,
            memory: &RunningCoachMemory,
        ) -> Result<(), DomainError> {
            *self.memory.lock().await = memory.clone();
            Ok(())
        }

        async fn store_running_coach_message(
            &self,
            msg: &RunningCoachMessage,
        ) -> Result<(), DomainError> {
            self.messages.lock().await.push(msg.clone());
            Ok(())
        }

        async fn get_user_by_id(&self, _user_id: Uuid) -> Result<User, DomainError> {
            Ok(self.user.clone())
        }

        async fn get_identity_profile(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<IdentityProfile>, DomainError> {
            Ok(None)
        }

        async fn get_athlete_profile(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<AthleteProfile>, DomainError> {
            Ok(None)
        }

        async fn get_activities_in_range(
            &self,
            user_id: Uuid,
            _from: chrono::DateTime<Utc>,
            _to: chrono::DateTime<Utc>,
        ) -> Result<Vec<Activity>, DomainError> {
            Ok(vec![Activity {
                id: Uuid::new_v4(),
                user_id,
                strava_id: 1,
                name: "Easy run".to_string(),
                sport_type: "Run".to_string(),
                start_date: Utc::now() - Duration::days(1),
                elapsed_time: 3600,
                moving_time: 3500,
                distance: 10_000.0,
                total_elevation_gain: 100.0,
                average_speed: 2.85,
                max_speed: 4.4,
                average_heartrate: None,
                max_heartrate: None,
                average_cadence: None,
                average_watts: None,
                calories: None,
                tag: domain::ActivityTag::Normal,
                summary_polyline: None,
                workout_type: None,
                streams_fetched_at: None,
                created_at: Utc::now(),
            }])
        }

        async fn get_activities(
            &self,
            user_id: Uuid,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<Activity>, DomainError> {
            self.get_activities_in_range(user_id, Utc::now() - Duration::days(7), Utc::now())
                .await
        }

        async fn list_running_coach_messages(
            &self,
            _user_id: Uuid,
            _limit: i64,
        ) -> Result<Vec<RunningCoachMessage>, DomainError> {
            Ok(self.messages.lock().await.clone())
        }
    }

    struct FakeLlm {
        tool_results: Arc<Mutex<VecDeque<ToolCompletionResult>>>,
    }

    impl FakeLlm {
        fn new(tool_results: Vec<ToolCompletionResult>) -> Self {
            Self {
                tool_results: Arc::new(Mutex::new(VecDeque::from(tool_results))),
            }
        }
    }

    #[async_trait]
    impl LlmClient for FakeLlm {
        async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
            Ok(Vec::new())
        }

        async fn chat_completion(
            &self,
            _model: &str,
            _messages: Vec<ChatMessage>,
            _reasoning_effort: Option<&str>,
        ) -> Result<ChatCompletionResult, LlmError> {
            Ok(ChatCompletionResult {
                content: "fallback".to_string(),
                usage: usage(1, 1, 2, 0.001),
            })
        }

        async fn chat_completion_with_schema(
            &self,
            _model: &str,
            _messages: Vec<ChatMessage>,
            _json_schema: JsonSchemaDefinition,
            _reasoning_effort: Option<&str>,
        ) -> Result<ChatCompletionResult, LlmError> {
            Ok(ChatCompletionResult {
                content: "{\"meaningful\":false,\"pinned_facts\":[],\"episodic_memory\":[]}"
                    .to_string(),
                usage: usage(0, 0, 0, 0.0),
            })
        }

        async fn chat_completion_with_tools(
            &self,
            _model: &str,
            _messages: Vec<ChatMessage>,
            _tools: Vec<ToolDefinition>,
            _tool_choice: Option<ToolChoice>,
            _parallel_tool_calls: Option<bool>,
            _reasoning_effort: Option<&str>,
        ) -> Result<ToolCompletionResult, LlmError> {
            let mut guard = self.tool_results.lock().await;
            guard
                .pop_front()
                .ok_or_else(|| LlmError::Api("No fake tool result queued".to_string()))
        }
    }

    struct FakeToolExecutor {
        definitions: Vec<ToolDefinition>,
        outputs: Arc<Mutex<VecDeque<String>>>,
        call_count: Arc<Mutex<usize>>,
    }

    impl FakeToolExecutor {
        fn new(outputs: Vec<String>) -> Self {
            Self {
                definitions: vec![ToolDefinition {
                    name: "search_sessions".to_string(),
                    description: "search".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": { "query": { "type": "string" } },
                        "required": ["query"]
                    }),
                }],
                outputs: Arc::new(Mutex::new(VecDeque::from(outputs))),
                call_count: Arc::new(Mutex::new(0)),
            }
        }

        async fn calls(&self) -> usize {
            *self.call_count.lock().await
        }
    }

    #[async_trait]
    impl CoachToolExecutor for FakeToolExecutor {
        fn tool_definitions(&self) -> Vec<ToolDefinition> {
            self.definitions.clone()
        }

        async fn execute_tool_call(
            &self,
            _user_id: Uuid,
            _call: &ToolCall,
        ) -> Result<String, DomainError> {
            let mut count = self.call_count.lock().await;
            *count += 1;
            drop(count);

            let mut outputs = self.outputs.lock().await;
            Ok(outputs
                .pop_front()
                .unwrap_or_else(|| "{\"ok\":true}".to_string()))
        }
    }

    fn tool_call(id: &str, name: &str, arguments: serde_json::Value) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments_raw: arguments.to_string(),
            arguments,
            arguments_parse_error: None,
        }
    }

    fn usage(prompt: u32, completion: u32, total: u32, cost: f64) -> LlmUsage {
        LlmUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
            cost,
        }
    }

    #[tokio::test]
    async fn tool_loop_single_call_then_final_answer() {
        let user_id = Uuid::new_v4();
        let store = FakeStore::new(user_id);
        let coach = CoachMemory::new(store);
        let llm = FakeLlm::new(vec![
            ToolCompletionResult {
                content: None,
                tool_calls: vec![tool_call(
                    "call_1",
                    "search_sessions",
                    serde_json::json!({ "query": "last run" }),
                )],
                finish_reason: Some("tool_calls".to_string()),
                usage: usage(10, 4, 14, 0.01),
            },
            ToolCompletionResult {
                content: Some("Use the Tuesday run as baseline.".to_string()),
                tool_calls: vec![],
                finish_reason: Some("stop".to_string()),
                usage: usage(6, 5, 11, 0.02),
            },
        ]);
        let executor = FakeToolExecutor::new(vec!["{\"matches\":[]}".to_string()]);

        let result = coach
            .send_message_with_tools(&llm, user_id, "How did my last run look?", &executor)
            .await
            .expect("coach response");

        assert_eq!(result.content, "Use the Tuesday run as baseline.");
        assert_eq!(result.total_tokens, 25);
        assert!((result.cost - 0.03).abs() < f64::EPSILON);
        assert_eq!(executor.calls().await, 1);
    }

    #[tokio::test]
    async fn tool_loop_multi_step_chain() {
        let user_id = Uuid::new_v4();
        let store = FakeStore::new(user_id);
        let coach = CoachMemory::new(store);
        let llm = FakeLlm::new(vec![
            ToolCompletionResult {
                content: None,
                tool_calls: vec![tool_call(
                    "call_1",
                    "search_sessions",
                    serde_json::json!({ "query": "last race" }),
                )],
                finish_reason: Some("tool_calls".to_string()),
                usage: usage(7, 3, 10, 0.01),
            },
            ToolCompletionResult {
                content: None,
                tool_calls: vec![tool_call(
                    "call_2",
                    "get_session_detail",
                    serde_json::json!({ "activity_id": Uuid::new_v4().to_string() }),
                )],
                finish_reason: Some("tool_calls".to_string()),
                usage: usage(6, 2, 8, 0.01),
            },
            ToolCompletionResult {
                content: Some("Race pacing faded after halfway.".to_string()),
                tool_calls: vec![],
                finish_reason: Some("stop".to_string()),
                usage: usage(5, 5, 10, 0.02),
            },
        ]);
        let executor = FakeToolExecutor::new(vec![
            "{\"matches\":[{\"activity_id\":\"abc\"}]}".to_string(),
            "{\"description_markdown\":\"# Session\"}".to_string(),
        ]);

        let result = coach
            .send_message_with_tools(&llm, user_id, "Analyze my last race session", &executor)
            .await
            .expect("coach response");

        assert_eq!(result.content, "Race pacing faded after halfway.");
        assert_eq!(executor.calls().await, 2);
    }

    #[tokio::test]
    async fn tool_loop_stops_after_max_iterations() {
        let user_id = Uuid::new_v4();
        let store = FakeStore::new(user_id);
        let coach = CoachMemory::new(store);

        let repeated = ToolCompletionResult {
            content: None,
            tool_calls: vec![tool_call(
                "call_1",
                "search_sessions",
                serde_json::json!({ "query": "something" }),
            )],
            finish_reason: Some("tool_calls".to_string()),
            usage: usage(1, 1, 2, 0.001),
        };
        let llm = FakeLlm::new(vec![
            repeated.clone(),
            repeated.clone(),
            repeated.clone(),
            repeated,
        ]);
        let executor = FakeToolExecutor::new(vec![
            "{\"matches\":[]}".to_string(),
            "{\"matches\":[]}".to_string(),
            "{\"matches\":[]}".to_string(),
            "{\"matches\":[]}".to_string(),
        ]);

        let err = coach
            .send_message_with_tools(&llm, user_id, "Keep searching", &executor)
            .await
            .expect_err("expected max-iteration failure");

        match err {
            DomainError::Internal(msg) => {
                assert!(msg.contains("exceeded"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn tool_loop_persists_compact_recent_tool_results() {
        let user_id = Uuid::new_v4();
        let store = FakeStore::new(user_id);
        let coach = CoachMemory::new(store);
        let llm = FakeLlm::new(vec![
            ToolCompletionResult {
                content: None,
                tool_calls: vec![tool_call(
                    "call_1",
                    "search_sessions",
                    serde_json::json!({ "query": "Lunch run" }),
                )],
                finish_reason: Some("tool_calls".to_string()),
                usage: usage(4, 2, 6, 0.01),
            },
            ToolCompletionResult {
                content: Some("That session looked controlled.".to_string()),
                tool_calls: vec![],
                finish_reason: Some("stop".to_string()),
                usage: usage(4, 4, 8, 0.01),
            },
        ]);

        struct SummarizingExecutor;

        #[async_trait]
        impl CoachToolExecutor for SummarizingExecutor {
            fn tool_definitions(&self) -> Vec<ToolDefinition> {
                vec![ToolDefinition {
                    name: "search_sessions".to_string(),
                    description: "search".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": { "query": { "type": "string" } },
                        "required": ["query"]
                    }),
                }]
            }

            async fn execute_tool_call(
                &self,
                _user_id: Uuid,
                _call: &ToolCall,
            ) -> Result<String, DomainError> {
                Ok(
                    "{\"matches\":[{\"activity_id\":\"93d3cd28-a734-4b25-9e5d-113ee5f640a7\",\"name\":\"Lunch Run\",\"start_date\":\"2026-03-03 10:49:49 UTC\"}]}"
                        .to_string(),
                )
            }

            fn summarize_tool_result(
                &self,
                _call: &ToolCall,
                _tool_output: &str,
            ) -> Option<String> {
                Some(
                    "search_sessions(query='Lunch run') -> 1 match: Lunch Run on 2026-03-03 (93d3cd28-a734-4b25-9e5d-113ee5f640a7)"
                        .to_string(),
                )
            }
        }

        let executor = SummarizingExecutor;

        coach
            .send_message_with_tools(&llm, user_id, "Show me my lunch run", &executor)
            .await
            .expect("coach response");

        let saved_memory = coach.store.saved_memory().await;
        assert_eq!(saved_memory.data.recent_tool_results.len(), 1);
        assert!(saved_memory.data.recent_tool_results[0].contains("Lunch Run"));
        assert!(saved_memory.data.recent_tool_results[0]
            .contains("93d3cd28-a734-4b25-9e5d-113ee5f640a7"));
    }
}
