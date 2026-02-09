# The summary page


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