# coord-paths.py
# Created by Michael Zhang

import yaml

with open("assets/projection-coords-flatten.yaml", "r") as f:
    try:
        source = yaml.safe_load(f)
    except yaml.YAMLError as e:
        print(e)

room_1 = input("Enter room 1   >>> ")
crossing = input("Enter crossing >>> ")
room_2 = input("Enter room 2   >>> ")

if room_1 not in source:
    print(f"Room {room_1} not found!")
    exit(1)
if room_2 not in source:
    print(f"Room {room_2} not found!")
    exit(1)
if crossing not in source:
    print(f"Crossing {crossing} not found!")
    exit(1)

x1 = source[room_1][0]
x2 = source[room_2][0]
xc = source[crossing][0]

print("  - dist:", abs(x1 - xc))
print("    nodes:")
print("    -", room_1)
print("    -", crossing)
print("    type: 0")
print("  - dist:", abs(x2 - xc))
print("    nodes:")
print("    -", crossing)
print("    -", room_2)
print("    type: 0")