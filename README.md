A web-app for runners who enjoys to analyze their data, to get more insight on their race performances, and help them
improve their training.

## Interval Parsing Algorithm

The core feature of this application is an intelligent interval parsing algorithm that automatically analyzes running
sessions tagged as "interval" workouts. The algorithm processes raw GPS and sensor data from Strava to identify and
characterize interval repetitions.

**Pipeline Overview:**

1. **Preprocessing**: Smooths speed data using rolling median and detects pauses (stops) using Strava's moving flag or
   speed thresholds.

2. **Segmentation**: Uses k-means clustering (k=2) to automatically determine a threshold between work and recovery
   speeds. Applies hysteresis labeling to create stable work/recovery segments, then cleans up GPS noise by absorbing
   short pauses and filtering out spurious segments.

3. **Rep Building**: Pairs work segments with their following recovery periods, labels warmup/cooldown phases, and
   computes quality metrics including pace consistency, steadiness, and fade (speed drop-off).

4. **Intensity Analysis**: Calculates percentage of Maximum Aerobic Speed (%MAS) for each repetition when MAS is
   available.

5. **Scoring**: Generates an interval quality score based on the number of reps, work/recovery alternation pattern, and
   speed consistency across repetitions.

The algorithm handles real-world GPS artifacts, varying recovery styles (jogging, walking, or standing), and produces
structured interval data suitable for detailed performance analysis.
