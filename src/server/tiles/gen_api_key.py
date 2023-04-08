'''Generates a simple ascii API key.'''

import string
import random

letters = string.ascii_letters  # upper and lowercase letters
system_random = random.SystemRandom()
print(''.join(system_random.choice(letters) for i in range(23)))
