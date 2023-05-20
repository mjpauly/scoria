# Bite-Sized Feature: Time Range Picker

The time range picker is a component used for selecting the range of times to
view in the map tab.

## (Warmup) Button for All Data

Implement a button to select a time range that includes all data a user has
logged. Do this by setting the start time to a value that is before Epsilon
exists, such as Jan 01, 2022. We will not worry about setting it to the actual
time of the first logged datapoint for now.

Place this button next to the existing options for selecting the time ranges of
today, the past 24 hours, and the past 7 days. You may need to abbreviate
"hours" and "days" to "h" and "d" to get the new button to fit on the same line.
Use the iPhone mini as the benchmark for this sizing.

## Quickly Adjust Time by Fixed Offsets

Currently, the start and end times can be set through the
browser-supported date and time picker. This is somewhat clunky to use for
quickly adjusting the time by a fixed amount, e.g. subtracting an hour
from the start time.

Implement a convenient method to step through time values by a selectable
offset. The desired offset sizes that can be selected are:

1s 10s 1m 10m 1h 1d 1w 1m 1y

We want it to be one click for adding or subtracting the selected offset, and
for it to be one click to adjust the offset size by one step. All new buttons
should fit on one line underneath the displayed time. A design that meets these
requirements is:

```
+-------------------------------------------+
|       05/10/2023        11:40 AM          |  # selected time
+-------------------------------------------+

 +------+ - - +------+    +------+  +------+
 |  1m  | 10m |  1h  |    | -10m |  | +10m |
 +------+ - - +------+    +------+  +------+

     ^            ^           ^         ^
     |            |           |         |
buttons to select the     buttons to subtract
next smaller or larger    or add time by the
offset size               selected offset size
```

Implementation notes:

- The selected offset sizes can reside in use_state() hooks
- The -offset and +offset buttons are given onclick callback functions, which
    implement the time adjustment logic
- The offset sizes for the start and end times should be independent
- To reduce duplicated code, you can refactor an individual time picker into its
    own component. The component would be passed a callback which is used to
    notify the parent of changes to the time, as well as the time value to
    display.
- The offset size type can be an enum you define, with a method to retrieve a
    `time::Duration` to add/subtract from the time given a specific enum
    variant, an implementation for displaying the enum variant as a string so we
    can show it to the user, and methods for getting the next smaller or larger
    offset size if it exists.

