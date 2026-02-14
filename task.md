# Calender-view

First, read @AGENTS.md

Currently, all the strava activities are listed in the main activities page as a list. While having a list view is
great, I want to also have a possible weekly-calendar view.

This view should be displayed as follow

- Each week is 1 row, where each day is 1 cell on this row
- Each "day"-cell contains a quick visual description of the activity done, so basically the distance, whether it is an
  internval, and the title that can be displayed very smally.
- The week-row should contain to the left a "weekly-recap" that simply puts the cumulative distance and cumulative time.

The user should always have the choice to pick which view he wants: there should be a "switch to list view" or "switch
to calendar view".

You don't have to optimize space, the weeks can be tall enough, so that we have enough space to display the days. 

# LLM settings

First, read @AGENTS.md

Your next task is going to add some default settings to the LLM-interaction features.

When clicking on "Continue to chat" on the AI-insights pop-up, the user should be prompted two settings

- to pick the LLM model from the available LLMs models. These models are defined in the backend's LLMClient
  implementation, see @backend/llm/src/lib.rs
- to pick the conversation length in context (number of messages). You will then need to modify the
  @backend/bin/src/helpers/conversation_manager.rs to reflect this setting.

# Custom LLM Chat

Currently, the user can only ask the LLM for a single answer and can't even decide on the question.

I want you to add a button at the bottom of the AI insight chat panels, that says "continue to chat". If the user clicks
this button, he is redirected to a new page (not a pop-up anymore, that is a full-page chatbox with the LLM)

You will need to create a new Rust model for conversations, something named `ConversationManager`, that will properly¨
call the messages of the LLM and feed all of them in the `LLMClient` as the `messages` parameter.

I believe that the "ai-chat" page should be a different model as the AI-insight, as the insights are really 1 answer. So
basically, we can create AI-chats from AI-insights, but the reverse is not true. For every AI-insights, I want to keep
track of the cost of the current chat, since the users will typically be billed for this. The openrouter response should
contain the following fields, so this is where we get the cost and token information. This information should be
displayed on the AI-chat page.

```
{
  "object": "chat.completion",
  "usage": {
    "completion_tokens": 2,
    "completion_tokens_details": {
      "reasoning_tokens": 0
    },
    "cost": 0.95,
    "cost_details": {
      "upstream_inference_cost": 19
    },
    "prompt_tokens": 194,
    "prompt_tokens_details": {
      "cached_tokens": 0,
      "cache_write_tokens": 100,
      "audio_tokens": 0
    },
    "total_tokens": 196
  }
}


Key Usage Fields
    total_tokens: Total tokens used in the request
    prompt_tokens: Tokens in your input messages
    completion_tokens: Tokens in the model's response
    reasoning_tokens: Tokens used for reasoning (for reasoning models)
    cached_tokens: Tokens read from cache
    cache_write_tokens: Tokens written to cache
    cost: Total cost in credits charged to your account
```

Finally, we also need to add a tab to the frontend, that says "AI chats", that simply lists all of the previous chats
that were done and a way to go back to them.

# Strava compliance

Strava API terms are rather strict, and I need to make some updates of the code as to be compliant. These are the
two rules that we have to respect.

- Do not keep any "Strava Data" for more than maximum 7 days. The strava data includes any data that comes directly from
  Strava. This includes activity names, streams, total distance, avg. speed.
- Do not ever store any GPS data

Basically, this will be quite impossible to respect, but I want us to make a tentative attempt of making the following
changes

- We should never store the GPS data to database: only query on demand when user is viewing the activity page.
- Regarding the strava stream, we should have a caching system configurable of N hours, default should be 24 hours.
- The activity name can be stored, alongside metadta of the activity (*this is just for UX, I know it breaks compliance
  but I will still try my best.*)
- The output of the interval parsing algorithm should be stored in the database persistently. I am pretty sure that
  currently, we don't store anything in database, so this is fine to leave this for the future.

By looking at our current database scheme (defined in @backend/storage/src/traits.rs and in actual implementation
@backend/storage/src/sqlite.rs) I want you to make a plan of the code changes to implement the points above.

Do ask me any questions if you have any.

# LLM integration part I

We now have a rather nice app, and I want to add the first LLM integration. This will consist in a few "precomputer"
LLM prompts. The users will have the option to request 1 LLM answer, for these predefined prompts.

The 1st pre-defined prompt are going to be related to the training feature. Basically, I want to add a few possible
prompts to the training page. These are the predefined prompts that I want:

1. Give me a critical overview of my training so far
2. Give me 3 suggestions for future interval trainings.

Under the hood, the LLM will be given a lot of context. They will get:

- Description of the current training plan (including the dates)
- The current date
- Total running volume since the beginning of the plan (split by weeks)
- Description of all the interval sessions that are in the plan (Note that currently, we have the output of the
  interval-parsing algorithm. We need a way to represent that as text. Just including the reps is enough, so really like
  what is displayed on the frontend)

They will be prompted to make just one answer containing everything!

Regarding the front-end, there will be a bit of work. The first thing is going to add a button to the training page for
each of the two prompts that I provided. It should be in a section called LLM Insights. Once the user clicks one of
these buttons, a pop-up should open that displays a chat box. This is actually going to be a fake chat box because the
message showed as sent by the user will be just a short description, and not the full message. There should be a
dropdown that shows something like: "Reveal LLM prompt" that shows the full message. Ff course, the LLM answer should be
displayed fully, unlike the prompt.

Regarding the technical implementation of the LLM chat, we will be using a library that is called Open Router I will
provide you a minimal Rust implementation of a Rust client to the Open Router API. This client supports something
called Chat Completion, which I think is what we need for the first proof of concept since it works for just one prompt,
one answer. However, feel free to modify this as you want. Note that this is hidden behind the trade so that we can swap
the actual open router implementation with just a dummy implementation as to not use too many tokens.

This template LLM client is defined in a new crate: here @backend/llm/src/lib.rs

# Training improvement

Our next task is going to improve the training feature of the database and also of the front-end. Essentially, a
training is just a set of activities that have been tagged as intervals joined together currently, a training only has a
name and a description. These trainings will be the backbones of the LLM integration. They will be given as an enriched
textual description as input to the LLM chat.I want you to add some more data to these trainings. So basically I want to
add:

- A start date
- An end date
- A race goal
  These new fields should be optional. The race goal should have predefined values and the option to add a custom goal.

# Strava API

https://www.strava.com/legal/api

# The profile page

I have made some progress without you: I added a "training" page to create and manage trainings, and a "race" page
that estimates the user's MAS.

Your next task is to add a "profile-page". This page will display the user profile, as well as some statistics about
his/her activities.

I want to the following data to be displayed

- year-to-date running distance, time and elevation, and average speed, number of intervals sessions
- current year running distance, time and elevation, and average speed, number of intervals sessions
- last-year running distance, time and elevation, and average speed
- total running distance, time and elevation, and average speed

For the POC, I don't really care about the storing strategy. If it makes it easier, we can derive these values on demand
everytime the frontend opens the profile page. If you think it is not much work to already add DB persistency,. then
let's do it. I would go a for sub-optimal approach which is to always recompute these values whenever `upsert_activity`
is called. If you change the DB scheme, you can assume that we can delete the db (no need to take care of data
migration)

# The race page

We will now create a new page: the MAS estimation page, or also called the "race page". This page will display the races
used to estimate the MAS, and an estimate of the MAS over time.

Users needs to provide "races", which are activities where the users declare that the full activity was done with
a max. efffort for this given effort.

The page name can be "Races (estimators)" for now.

Simialry as the training page, it should be possible to add some activities to this "group" of activities. Basically,
this page should list all of the activities that are tagged as "race". Since users have the ability to un-tag an
activity this leaves them the choice to opt-out one race for which they don't feel that it should impact the performance
estimator.

The race page should display all the associated races, and a timeline of the estimated MAS overtime.

Regarding the MAS estimator, I want you to use this POC formula.

```
### Primitive MAS estimate from one race

Let:
- `D_m` = race distance in meters
- `T_s` = race time in seconds
- `p`   = assumed fraction of MAS for that race (e.g., 0.90 for a 10k)

Average speed:
\[
v = \frac{D_m}{T_s} \quad (\text{m/s})
\]

MAS in km/h:
\[
\text{MAS}_{km/h} \approx \frac{v \cdot 3.6}{p}
= \frac{D_m}{T_s}\cdot\frac{3.6}{p}
\]

(Equivalent) MAS in m/s:
\[
\text{MAS}_{m/s} \approx \frac{D_m}{T_s}\cdot\frac{1}{p}
\]

where p by race distance (simple defaults)

1500m–3k: 
p=0.98

5k: 
p=0.92

10k: 
p=0.90

Half-marathon: 
p=0.85

Marathon: 
p=0.80
```

# Add the training hug / intervals page

Currently, the website list many Strava activites, and allows to click on each activity and to see the data, the
streams,
and the result of the interval parsing algorithm.

Your task is going to be add a new concept to our project: trainings. Trainings are a collection of activites tagged as
intervals.

Note that there is currently no such thing as a "program", neither in the frontend or in the backend. Therefore, you
will need to create this new model object in the backend, and to create the frontend components to update the trainings.

I want you to make the following changes to the frontend:

- Add a new page (the trainings page) that list all of the trainings, and allows to create a new one.
- Add a new page (the training page) that list all of the intervals of a given training.
- When displaying 1 interval (the activity page), there should be the option to add them to multiple trainings and a
  list
  of all the trainings associated to this activity
- Ideally, you should also change the "activities" page to filter by activity tag, so that we can show only the
  intervals.

Notes

- only "interval-tagged" sessions can be added to a training.

If you have any questions, ask them now