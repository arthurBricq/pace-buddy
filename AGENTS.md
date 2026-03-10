I am a self-trained runner, and I often use data-science and LLMs to improve my running performance, by getting
help for my training sessions. This project helps me to do that.

This project contains the code of a web-app that provides advanced analysis tools for runners who want to
monitor their running performance, especially targeted for interval training and runners who are following a training
program. It also provides an interface to talk with LLM that are directly contextualized to the user's training
sessions, in a plain-text format.

Here is the high-level architecture of the project:

- The backend is written in Rust, see the entry point in `backend/Cargo.toml` and `backend/bin/src/main.rs`. The
  frontend is written in React.
- We use Strava as the source of data with their API. All of the data loaded from strava is stored in a sqlite
  database, as to avoid the need to fetch many times the same data.
- Authentication and account linking are done via Strava OAuth; no local password or passkey flow is required.

# Features

## Interval parsing algorithm

One of the core value of this project is the **interval parsing algorithm** (see the crate defined at
`backend/intervals/src/lib.rs`), which allows every session tagged as `interval` by the user to be parsed into a
series of intervals, which is something with high-descriptive value of the session.

## Trainings

The users are able to create "training programs" where they will be can group sessions into a training program,
and their LLM trainer will be able to train on these programs. The users are then able to request an AI-insights from
their trainings, and AI insights can be transformd into "AI chat"s, where the users can simply talk with the AI.

## Contextualized LLM chats

Users don't have to create AI-chats from training, they can also create chats from scratch. When they do this, there is
a "add context" panel, where users can select what they want to feed the LLM with.

## Development Node

This project is under development, therefore, I don't care about database migrations. For any database modification, you
can assume that we can drop the database and start from scratch.
