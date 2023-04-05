import numpy as np
import matplotlib.cm as cm

def float2byte(x):
    return int(np.round(x * 255))

def floats2bytes(x):
    return list(map(lambda a: float2byte(a), x))

def print_cmap(c):
    for i in range(len(c)):
        d = floats2bytes(c[i])
        print('({}, "rgb({},{},{})"),'.format(i / (len(c) - 1), d[0], d[1], d[2]))

# print_cmap(cm.viridis.colors)
# print_cmap(cm.plasma.colors)
print_cmap(cm.twilight.colors)
