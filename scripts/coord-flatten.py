# coord-flatten.py
# Created by Michael Zhang

# This script flattens projection-coords.yaml file.


import yaml

with open("assets/projection-coords.yaml", "r") as f:
    try:
        source = yaml.safe_load(f)
    except yaml.YAMLError as e:
        print(e)

result = {}

for floor_key in source:
    for side_key in source[floor_key]:
        if source[floor_key][side_key] != None:
            for i in source[floor_key][side_key]:
                result[i] = source[floor_key][side_key][i]

with open("assets/projection-coords-flatten.yaml", "w") as f:
    yaml.safe_dump(result, f)
