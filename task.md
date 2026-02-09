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