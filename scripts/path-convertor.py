# path-convertor.py
# Created by Michael Zhang

import yaml

with open("assets/paths.yaml", "r") as f:
    try:
        source = yaml.safe_load(f)
    except yaml.YAMLError as e:
        print(e)

for floor_key in source:
    for side_key in source[floor_key]:
        if source[floor_key][side_key] != None:
            for i in source[floor_key][side_key]:
                print(i["nodes"][0], i["nodes"][1], i["dist"], i["type"])