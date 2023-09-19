# coord-generator.py
# Created by Michael Zhang

# This script generates the x-coordinates of the nodes
# by calculating the midpoint using the rooms' lengths.

A_BLOCK = False
FIRST_OFFSET = 540
LAST_OFFSET = 40

rooms = []
lengths = []
with open('assets/projection-length.yaml', 'r') as f:
    for line in f.readlines():
        room, length = line.split(': ')
        length = int(length)
        rooms.append(room)
        lengths.append(length)


if A_BLOCK:
    print("A block. Total length:", sum(lengths) + FIRST_OFFSET)
    if FIRST_OFFSET != 0:
        print(f"\033[1;34mNOTE: The first node offset is set to {FIRST_OFFSET}.\033[0m")
    if sum(lengths) + FIRST_OFFSET != 2290:
        print("\033[1;33mWARNING: Total length is not 2290!\033[0m")
    for i in rooms:
        if i[0] == 'B':
            print("\033[1;33mWARNING: Using A block mode, but data contains B block rooms!\033[0m")
            break
    for i in range(len(rooms)):
        print(f'{rooms[i]}: {FIRST_OFFSET+int(sum(lengths[:i]) + lengths[i] / 2)}')
else:
    print("B block. Total length:", sum(lengths))
    if LAST_OFFSET != 0:
        print(f"\033[1;34mNOTE: The last node offset is set to {LAST_OFFSET}.\033[0m")
    for i in rooms:
        if i[0] == 'A':
            print("\033[1;33mWARNING: Using B block mode, but data contains A block rooms!\033[0m")
            break
    for i in range(len(rooms)):
        print(f'{rooms[i]}: {2290-LAST_OFFSET-int(sum(lengths[:i]) + lengths[i] / 2)}')