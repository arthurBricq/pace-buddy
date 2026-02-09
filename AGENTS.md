This project contains the code of a web-app that provides advanced analysis tools for runners who want to
monitor their running performance, especially targeted for interval training and runners who are following a training
program.

Here is the high-level architecture of the project:

- The backend is written in Rust, see the entry point in `backend/Cargo.toml` and `backend/bin/src/main.rs`. The
  frontend is written in React.
- We use Strava as the source of data with their API. All of the data loaded from strava is stored in a sqlite
  database, as to avoid the need to fetch many times the same data.
- The registration is done merely via passkeys, as to avoid the need to store any sensitive data like passwords.

# Vision of the future

The core value of this project is the **interval parsing algorithm** (see the crate defined at
`backend/intervals/src/lib.rs`), which allows every session tagged as `interval` by the user to be parsed into a
series of intervals, which is something with high-descriptive value of the session.

The goal will be to allow the users to chat with LLMs, using an openrouter account. This LLM, called the LLM trainer,
will have access to a lot of descriptive data (in textual form), allowing the user to have a very informative chat with
his LLM trainer.

The user will be able to create "training programs", where they will be able to group sessions into a training program,
and their LLM trainer will be able to train on these programs. This provides a much more "customized" LLM experience
than Strava's LLM message dispalyed after each session.



