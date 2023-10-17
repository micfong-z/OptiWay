# path-generator.py
# Created by Michael Zhang

# This script generates paths on the same floor on one block
# using data from assets/projection-coords.yaml

# Path types:
# 0: Normal path between two rooms / between a room and one end of a bridge
# 1: Bridge between two buildings
# 2: Normal staircases (e.g. S2-3)
# 3: Spiral staircases between two floors only (e.g. S2-2)
# 4: Other types of staircases (e.g. S4-6)

import yaml  # well yeah I just love YAML ;)

result = {}

with open("assets/projection-coords.yaml", "r") as f:
    try:
        source = yaml.safe_load(f)
    except yaml.YAMLError as e:
        print(e)

for floor_key in source:
    result[floor_key] = {}
    for block in ["A", "B"]:
        result[floor_key][block] = []
        for room_idx in range(len(source[floor_key][block].keys()) - 1):
            room_keys = list(source[floor_key][block].keys())
            x1 = source[floor_key][block][room_keys[room_idx]][0]
            x2 = source[floor_key][block][room_keys[room_idx + 1]][0]
            dist = abs(x2 - x1)
            result[floor_key][block].append(
                {"nodes": (room_keys[room_idx], room_keys[room_idx+1]), "dist": dist, "type": 0})

try:
    with open("assets/paths.yaml", "r") as f:
        response = input("assets/paths.yaml already exists. Overwrite? (y/n) ")
        if response == "y":
            print("Overwriting assets/paths.yaml...")
        else:
            print("Aborted.")
            exit()
except FileNotFoundError:
    pass

with open("assets/paths.yaml", "w") as f:
    yaml.safe_dump(result, f)
