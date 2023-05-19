"""Export the colormaps from matplotlib.

We produce an array of (float, color_string) values. The floats are selected so
that a color can be chosen if the normalized value to color by compares LESS
than the value in the array. For example, a colormap with 4 values in it gets
exported with threshold values of: array([0.25, 0.5 , 0.75, 1.  ]). Thus we have
4 bins inside the range [0, 1] into which values can fall, where the bin is
defined by its upper bound. In this fashion, we can find the color to used based
on the sorted insertion index of the value to colormap. A value less than 0.25
would get inserted at index 0 to maintain sorted order, so we take the color
at index 0. Values greater than 1 will have an insertion index of n, which is
invalid for zero-indexed arrays, so make sure to clamp it to n-1.
"""
import numpy as np
import matplotlib.cm as cm
from matplotlib.colors import to_hex

def print_cmap(c):
    n = len(c)
    thresholds = np.linspace(1/n, 1, n)
    for i in range(len(c)):
        print('({}, "{}"),'.format(thresholds[i], to_hex(c[i])))

# print_cmap(cm.viridis.colors)
# print_cmap(cm.plasma.colors)
print_cmap(cm.twilight.colors)
