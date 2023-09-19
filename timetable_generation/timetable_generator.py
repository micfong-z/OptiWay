"""
Return File Structure (JSON):
{
    student_number1 (string): {
        day (string): {
            period (string): room (string)
        }
        ...
    }
}

Example:
{
    "21447": {
        "1": {
            "1": "531",
            "2": "531",
            "3": "430",
            "4": "430"
            ...
        },
        "2": {...},
        ...
    },
    "21448": {
        "1": {
            "1": "210",
            "2": "210",
            "3": "322",
            "4": "322"
            ...
        },
        "2": {...},
        ...
    }
}
"""

import random
import copy
import json


class TimetableGenerator:
    def __init__(self):
        # Init Data #
        # the frequency for elective classes
        self.glevel_elective_freq = {
            "Extra": 12,
            "Bus": 18,
            "Hum": 14,
            "Enr": 6,
            "Lan": 5,
            "Sci": 25
        }
        # the dict to map class to floor number
        self.floor_map = {
            "Extra": 1,
            "Bus": 2,
            "Hum": 3,
            "Enr": 4,
            "Lan": 5,
            "Chi": 5,
            "Eng": 5,
            # does not account for sci as it covers three floors
        }
        # all the available classrooms
        self.avail_rooms = {
            2: set(range(201, 242)) - {203, 209, 213, 218, 229, 230, 232, 234, 236, 237, 238, 240},
            3: set(range(301, 334)) - {303, 304, 309, 317, 321, 322, 328},
            4: set(range(401, 434)) - {403, 404, 412, 413, 418, 420, 422, 428},
            5: set(range(501, 535)) - {503, 512, 517, 518, 520, 527, 528, 529},
            6: set(range(601, 632)) - {603, 614, 615, 620, 623, 625, 626, 627},
            7: set(range(701, 734)) - {703, 714, 716, 720, 723, 725, 727, 729, 730, 732},
            8: set(range(801, 822)) - {808, 810, 813, 816, 818, 820}
        }
        # the dicts to map glevel schedule set to period number
        # formate: "<day><1st period><2nd period>"
        glevel_core_set_map = {
            0: ["1_7_8", "4_9_10"],
            1: ["1_9_10", "3_3_4"],
            2: ["2_1_2", "5_3_4"],
            3: ["2_3_4", "5_7_8"]
        }
        glevel_elect_set_map = {
            0: ["1_1_2", "4_3_4"],
            1: ["1_3_4", "2_9_10"],
            2: ["2_7_8", "4_7_8"],
            3: ["3_1_2", "5_1_2"]
        }
        self.glevel_set_map = [glevel_core_set_map, glevel_elect_set_map]
        # the currently filled rooms
        self.filled_rooms = [dict(zip(range(2, 9), [set() for _ in range(2, 9)])) for _ in range(9)]
        self.time_table = {}

    # map rooms into different buildings
    @staticmethod
    def map_building(room):
        if 200 < room < 221 or 300 < room < 320 or 400 < room < 421 or 500 < room < 521 or 600 < room < 618 or 700 < room < 717 or 800 < room < 811:
            return "A"
        else:
            return "B"

    def generate_g_level(self, grade):
        """
        Generate G-level classes
        1: Find the total number of classes for each of the categories: Extra(1F), Bus(2F), Hum(3F), Enr(4F), Lan(5F), Sci(6F)
            1.1 assume no 1-period classes -> 380 2-period classes per week -> 360 non-PE 2-period classes (500 / 25 = 20)
            1.2 180 non-repetitive non-PE courses (1 course = 2 classes per week)
            1.3 100 non-repetitive elective courses (core courses: chinese, eng, math, chem) -> true as everyone has 5 electives
                1.3.1 80 clean extra courses (there are one more 2-period english and enrichment class)
        2: distribute 80 courses into Extra, Bus, Hum, Enr, Lan, Sci following common-sense weightings (dict "elective_freq")
            2.1 Extra(art/drama/pe/music): 12
            2.2 Bus (econ/bus): 18
            2.3 Hum (geo+his+gp): 14
            2.4 Enr (cs): 6
            2.5 Lan (fren/span/ja): 5
            2.6 Sci (bio+phy): 25
        3: distribute timeslots for each course
           3.1 assign fixed scheduling
                - SCIE has fixed scheduling - it separates classes into different timeslot couples
                   e.g, courses at Monday P1 would repeat at Wednesday P1
                - 19 2-periods in a week, 18 non-PE, 2-periods in a week, 9 schedule sets,
                - 8 clean schedule sets (without enrichment and another eng)
                - schedule sets for core and electives are separated
                - hence, distribute each class into these sets
            3.2 define schedule sets
                - 4 core schedule sets (Enr, Chi, Eng, Sci)
                    1. Mon78, Thu90
                    2. Mon90, Wed34
                    3. Tue12, Fri34
                    4. Tue34, Fri78
                - 4 elective schedule sets (Extra, Bus, Hum, Lan)
                    1: Mon12, Thu34
                    2. Mon34, Tue90
                    3. Tue78, Thu78
                    4. Wed12, Fri12
        4: distribute the rooms for each course
            4.1 the rooms available for each floor is stored in the dict "avail_rooms"
            4.2 limitations: courses at each scheduling set should not have the same room
        5: distribute each student's list of courses taken
        6: project back to generate the timetable

        :param: grade: either G1 or G2
        :return: timetable as a JSON
        """
        # Procedures #
        # 3 distribute timeslots for each course, each slot only needs 20 courses
        core_slots = [[] for _ in range(4)]
        # assign the core classes
        for core in range(20):
            core_classes = ["Enr", "Chi", "Eng", "Sci"]
            for i in range(4):
                core_class = random.choice(core_classes)
                core_slots[i].append(core_class + "_core_" + str(core))
                core_classes.remove(core_class)
        # assign the elective classes
        elect_slots = [[] for _ in range(4)]
        avail_slots = list(range(4))
        for k, v in self.glevel_elective_freq.items():
            for i in range(v):
                slot = random.choice(avail_slots)
                elect_slots[slot].append(k + "_elect_" + str(i))
                if len(elect_slots[slot]) == 20:
                    avail_slots.remove(slot)

        # 4 distribute the rooms for each course
        if grade == "G1":
            slots = core_slots + elect_slots
        else:
            slots = elect_slots + core_slots
        rooms = {}
        filled_rooms = []  # track the filled rooms for each floor for each slot
        for index, slot in enumerate(slots):
            filled_room = dict(zip(range(2, 9), [set() for _ in range(2, 9)]))
            for course in slot:
                if course[:3] == "Sci":  # sci courses are distributed to three floors
                    floor = random.choice([6, 7, 8])  # TODO: change to greedy floor
                else:
                    floor = self.floor_map[course.split("_")[0]]
                    if floor == 1:  # if it is extra, just assign G
                        rooms[course] = "G"
                        continue
                # divide by the current filled rooms, use mod because the core slots for g1 are elective slots for g2
                room = random.choice(list(self.avail_rooms[floor] - filled_room[floor] - self.filled_rooms[(index + 4) % 8][floor]))
                rooms[course] = self.map_building(room) + str(room)
                filled_room[floor].add(room)
            filled_rooms.append(filled_room)

        # update the global filled_rooms
        for i in range(8):
            for j in range(2, 9):
                self.filled_rooms[i][j].update(filled_rooms[i][j])
                # [514, 519, 521, 523, 530, 531, 532, 533, 534, 502, 504, 508, 511]

        # 5 distribute each student's list of courses taken
        if grade == "G1":
            offset = 23000
        else:
            offset = 22000
        students = dict(zip(range(offset+1, offset+501), [[] for _ in range(500)]))
        # assign each student to a core class (#0 - 19)
        core_counts = [0 for _ in range(20)]  # track the number of students in each core class
        cores = list(range(20))
        for student in students:
            core = random.choice(cores)
            students[student].extend([f"Enr_core_{core}", f"Chi_core_{core}", f"Eng_core_{core}", f"Sci_core_{core}"])
            core_counts[core] += 1
            if core_counts[core] == 25:
                cores.remove(core)

        elect_slots_copy = copy.deepcopy(elect_slots)
        # assign each student to an elective class
        filled_courses = {i: dict(zip(elect_slots_copy[i], [0] * 20)) for i in range(4)}  # track the filled courses for each slot
        for student in students:
            for i, slot in enumerate(elect_slots_copy):
                course = random.choice(slot)
                students[student].append(course)
                filled_courses[i][course] += 1
                if filled_courses[i][course] == 25:
                    elect_slots_copy[i].remove(course)

        # 6 projection back
        timetable = {str(i): {str(j): {str(k): "G" for k in range(1, 11)} for j in range(1, 6)} for i in range(offset+1, offset+501)}
        filled_rooms.append(dict(zip(range(2, 9), [set() for _ in range(2, 9)])))  # additional slot to hold the extra english and PE class
        if grade == "G1":
            core_set = 0
            elect_set = 1
        else:
            core_set = 1
            elect_set = 0
        for k, student in students.items():
            for course in student[:4]:  # for the core classes
                for index, slot in enumerate(core_slots):
                    if course in slot:
                        day, p1, p2 = self.glevel_set_map[core_set][index][0].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = self.glevel_set_map[core_set][index][1].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                if course.split("_")[0] == 'Eng' and int(course.split("_")[-1]) <= 9:  # handle the extra english and PE class to spare slots
                    timetable[str(k)]['4']['1'] = rooms[course]
                    timetable[str(k)]['4']['2'] = rooms[course]
                    timetable[str(k)]['3']['7'] = 'G'
                    timetable[str(k)]['3']['8'] = 'G'
                    filled_rooms[-1][5].add(rooms[course])
                elif course.split("_")[0] == 'Eng' and int(course.split("_")[-1]) > 9:
                    timetable[str(k)]['4']['1'] = 'G'
                    timetable[str(k)]['4']['2'] = 'G'
                    timetable[str(k)]['3']['7'] = rooms[course]
                    timetable[str(k)]['3']['8'] = rooms[course]  # TODO: place G1 and G2 with un-collided English rooms
                    # add g2 extra classes
                    filled_rooms[-1][5].add(rooms[course])
            for course in student[4:]:  # for the elective classes
                for index, slot in enumerate(elect_slots):
                    if course in slot:
                        day, p1, p2 = self.glevel_set_map[elect_set][index][0].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = self.glevel_set_map[elect_set][index][1].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]

        self.filled_rooms[8][5].update(filled_rooms[8][5])
        self.time_table.update(timetable)

        return timetable

    def generate_as_al_level(self):
        """
        Generate AS-Level Classes
        1: Find the total number of classes for each of the categories: Extra(1F), Bus(2F), Hum(3F), Enr(4F), Lan(5F), Sci(6F)
            1.1 assume that each student take 1 Eng and 4 Electives -> 5*3=15 2-periods per week for a student
            1.2 5*500/25=100 2-periods courses in total
        2: distribute 100 courses into Extra, Bus, Hum, Enr, Lan, Sci following common-sense weightings (dict "as_freq")
            2.1 Extra (art/drama/pe/music): 4
            2.2 Bus (econ/acc): 10
            2.3 Hum (geo/his/psy): 10
            2.4 Mat (mat/further/cs): 23
            2.5 Eng (lit/lan/gc/3rd lan): 23
            2.6 Sci (bio/phy/chem): 30
        3: distribute timeslots for each course
            3.1 assign fixed scheduling
                - each slot accounts for 3 2-periods in AS
                - there are in total 6 slots for AS -> don't need to fill in every slot
            3.2 define schedule sets
                1. Mon12, Tue78, Thu34
                2. Mon34, Tue90, Thu78
                3. Mon78, Wed12, Thu90
                4. Mon90, Wed34, Fri12
                5. Tue12, Wed78, Fri34
                6. Tue34, Thu12, Fri56
        4: distribute the rooms for each course
            4.1 the rooms available for each floor is stored in the dict "avail_rooms"
            4.2 limitations: courses at each scheduling set should not have the same room
                - should check "filled_rooms" when handling this
                - hence, process "filled_rooms" into a timetable format
        5: distribute each student's list of courses taken
        6: project back to generate the timetable


        :return: timetable as a JSON
        """
        # dictionary for as course frequencies
        as_freq = {
            "Extra": 4,
            "Bus": 10,
            "Hum": 10,
            "Enr": 23,
            "Eng": 23,
            "Sci": 30
        }
        # dictionary for as course scheduling sets
        as_set_map = {
            0: ["1_1_2", "2_7_8", "4_3_4"],
            1: ["1_3_4", "2_9_10", "4_7_8"],
            2: ["1_7_8", "3_1_2", "4_9_10"],
            3: ["1_9_10", "3_3_4", "5_1_2"],
            4: ["2_1_2", "3_7_8", "5_3_4"],
            5: ["2_3_4", "4_1_2", "5_7_8"]
        }
        # Procedures #
        # 3 distribute timeslots for each course, each slot only needs 20 courses
        slots = [[] for _ in range(6)]
        avail_slots = list(range(6))
        for k, v in as_freq.items():
            for i in range(v):
                slot = random.choice(avail_slots)
                slots[slot].append(k + "_" + str(i))
                if len(slots[slot]) == 20:
                    avail_slots.remove(slot)

        # 4 distribute the rooms for each course
        # transform the g_level filled_rooms into a timetable format
        filled_timetable = {str(j): {str(k): set() for k in range(1, 11)} for j in range(1, 6)}
        glevel_set_map = list(self.glevel_set_map[0].values()) + list(self.glevel_set_map[1].values())
        for index, slot in enumerate(self.filled_rooms[:8]):  # don't include the extra two slots at last
            for v in slot.values():
                for room in v:
                    day, p1, p2 = glevel_set_map[index][0].split("_")
                    filled_timetable[day][p1].add(room)
                    filled_timetable[day][p2].add(room)
                    day, p1, p2 = glevel_set_map[index][1].split("_")
                    filled_timetable[day][p1].add(room)
                    filled_timetable[day][p2].add(room)
        for index, slot in enumerate(self.filled_rooms[8:]):  # process the last two extra slots
            for v in slot.values():
                for room in v:
                    filled_timetable['3']['7'].add(int(room[1:]))
                    filled_timetable['3']['8'].add(int(room[1:]))
                    filled_timetable['4']['1'].add(int(room[1:]))
                    filled_timetable['4']['2'].add(int(room[1:]))

        # 4 distribute the rooms for each course
        rooms = {}
        filled_rooms = []  # track the filled rooms for each floor for each slot
        for index, slot in enumerate(slots):
            day1, p11, p12 = as_set_map[index][0].split("_")
            day2, p21, p22 = as_set_map[index][1].split("_")
            day3, p31, p32 = as_set_map[index][2].split("_")
            filled_room = dict(zip(range(2, 9), [set() for _ in range(2, 9)]))
            for course in slot:
                if course[:3] == "Sci":  # sci courses are distributed to three floors
                    floor = random.choice([6, 7, 8])  # TODO: change to greedy floor
                else:
                    floor = self.floor_map[course.split("_")[0]]
                    if floor == 1:  # if it is extra, just assign G
                        rooms[course] = "G"
                        continue
                # divide by the current filled rooms, use mod because the core slots for g1 are elective slots for g2
                room = random.choice(
                    list(self.avail_rooms[floor] - filled_timetable[day1][p11] - filled_timetable[day2][p21] - filled_timetable[day3][p31]))
                rooms[course] = self.map_building(room) + str(room)
                filled_room[floor].add(room)
            filled_rooms.append(filled_room)

        # 5 distribute each student's list of courses taken
        # generate the students' slots
        slots_count = [[i, len(slot)*25] for i, slot in enumerate(slots)]
        offset = 21000
        students_slots = {i: [] for i in range(offset+1, offset+501)}
        for student in students_slots:
            slots_count = sorted(slots_count, reverse=True, key=lambda x: x[1])
            slots_indices = [item[0] for item in slots_count[:5]]
            students_slots[student] = slots_indices
            for i in range(6):
                if slots_count[i][0] in slots_indices:
                    slots_count[i][1] -= 1

        # print(students_slots)

        # generate the students
        students = {i: [] for i in range(offset+1, offset+501)}
        slots_copy = copy.deepcopy(slots)
        filled_courses = {i: dict(zip(slots_copy[i], [0] * len(slots_copy[i]))) for i in range(6)}  # track the filled courses for each slot
        for student in students:
            # print(student)
            for i in students_slots[student]:
                slot = slots_copy[i]
                course = random.choice(slot)
                students[student].append(course)
                filled_courses[i][course] += 1
                if filled_courses[i][course] == 25:
                    slots_copy[i].remove(course)
        
        # 5 generate the timetable
        timetable = {str(i): {str(j): {str(k): "G" for k in range(1, 11)} for j in range(1, 6)} for i in range(offset+1, offset+501)}
        for k, student in students.items():
            for course in student:
                for index, slot in enumerate(slots):
                    if course in slot:
                        day, p1, p2 = as_set_map[index][0].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = as_set_map[index][1].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = as_set_map[index][2].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
        
        """Generate AL Level, written in the same function to avoid passing filled_rooms which is hard for different set maps"""
        """
        Generate AL-Level Classes
        1: Find the total number of classes for each of the categories: Extra(1F), Bus(2F), Hum(3F), Enr(4F), Lan(5F), Sci(6F)
            1.1 assume that each student take 1 Eng and 3 Electives -> 4*3=12 2-periods per week for a student
            1.2 4*500/25=80 2-periods courses in total
        2: distribute 80 courses into Extra, Bus, Hum, Enr, Lan, Sci following common-sense weightings (dict "al_freq")
            2.1 Extra (art/drama/pe/music): 4
            2.2 Bus (econ/acc): 12
            2.3 Hum (geo/his/psy): 10
            2.4 Mat (mat/further/cs): 15
            2.5 Eng (lit/lan/gc/3rd lan): 23
            2.6 Sci (bio/phy/chem): 20
        3: distribute timeslots for each course
            3.1 assign fixed scheduling
                - each slot accounts for 3 2-periods in AL
                - the set map for AL is the same as that for AS
            3.2 define schedule sets
                1. Mon12, Tue78, Thu34
                2. Mon34, Tue90, Thu78
                3. Mon78, Wed12, Thu90
                4. Mon90, Wed34, Fri12
                5. Tue12, Wed78, Fri34
                6. Tue34, Thu12, Fri56
        4: distribute the rooms for each course
            4.1 the rooms available for each floor is stored in the dict "avail_rooms"
            4.2 limitations: courses at each scheduling set should not have the same room
                - should check "filled_rooms" when handling this
                - hence, process "filled_rooms" into a timetable format
        5: distribute each student's list of courses taken
        6: project back to generate the timetable


        :return: timetable as a JSON
        """
        al_freq = {
            "Extra": 4,
            "Bus": 12,
            "Hum": 10,
            "Enr": 15,
            "Eng": 23,
            "Sci": 20
        }

        # PROCEDURES #
        # 3 distribute timeslots for each course, each slot only needs 20 courses
        slots = [[] for _ in range(6)]
        avail_slots = list(range(6))
        for k, v in al_freq.items():
            for i in range(v):
                slot = random.choice(avail_slots)
                slots[slot].append(k + "_" + str(i))
                if len(slots[slot]) == 20:
                    avail_slots.remove(slot)

        # 4 distribute the rooms for each course
        rooms = {}
        for index, slot in enumerate(slots):
            day1, p11, p12 = as_set_map[index][0].split("_")
            day2, p21, p22 = as_set_map[index][1].split("_")
            day3, p31, p32 = as_set_map[index][2].split("_")
            filled_room = dict(zip(range(2, 9), [set() for _ in range(2, 9)]))
            for course in slot:
                if course[:3] == "Sci":  # sci courses are distributed to three floors
                    floor = random.choice([6, 7, 8])  # TODO: change to greedy floor
                else:
                    floor = self.floor_map[course.split("_")[0]]
                    if floor == 1:  # if it is extra, just assign G
                        rooms[course] = "G"
                        continue
                # divide by the current filled rooms, use mod because the core slots for g1 are elective slots for g2
                room = random.choice(
                    list(self.avail_rooms[floor] - filled_timetable[day1][p11] - filled_timetable[day2][p21] 
                         - filled_timetable[day3][p31] - filled_rooms[index][floor]))  # minus the as time slots as well
                rooms[course] = self.map_building(room) + str(room)
                filled_room[floor].add(room)
        
        # 5 distribute each student's list of courses taken
        # generate the students' slots
        slots_count = [[i, len(slot)*25] for i, slot in enumerate(slots)]
        offset = 20000
        students_slots = {i: [] for i in range(offset+1, offset+501)}
        for student in students_slots:
            slots_count = sorted(slots_count, reverse=True, key=lambda x: x[1])
            slots_indices = [item[0] for item in slots_count[:4]]
            students_slots[student] = slots_indices
            for i in range(6):
                if slots_count[i][0] in slots_indices:
                    slots_count[i][1] -= 1

        # print(students_slots)

        # generate the students
        students = {i: [] for i in range(offset+1, offset+501)}
        slots_copy = copy.deepcopy(slots)
        filled_courses = {i: dict(zip(slots_copy[i], [0] * len(slots_copy[i]))) for i in range(6)}  # track the filled courses for each slot
        for student in students:
            # print(student)
            for i in students_slots[student]:
                slot = slots_copy[i]
                course = random.choice(slot)
                students[student].append(course)
                filled_courses[i][course] += 1
                if filled_courses[i][course] == 25:
                    slots_copy[i].remove(course)
        
        # 5 generate the timetable
        al_timetable = {str(i): {str(j): {str(k): "G" for k in range(1, 11)} for j in range(1, 6)} for i in range(offset+1, offset+501)}
        for k, student in students.items():
            for course in student:
                for index, slot in enumerate(slots):
                    if course in slot:
                        day, p1, p2 = as_set_map[index][0].split("_")
                        al_timetable[str(k)][day][p1] = rooms[course]
                        al_timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = as_set_map[index][1].split("_")
                        al_timetable[str(k)][day][p1] = rooms[course]
                        al_timetable[str(k)][day][p2] = rooms[course]
                        day, p1, p2 = as_set_map[index][2].split("_")
                        al_timetable[str(k)][day][p1] = rooms[course]
                        al_timetable[str(k)][day][p2] = rooms[course]
        
        timetable.update(al_timetable)
        self.time_table.update(timetable)

        return timetable

    def add_pshe(self):
        student_list = list(self.time_table.keys())
        offset = 0
        flag = True
        for floor in self.avail_rooms.values():
            for room in floor:
                for i in range(offset, offset+25):
                    if i >= len(student_list):
                        flag = False
                        break
                    self.time_table[student_list[i]]['3']['5'] = self.map_building(room) + str(room)
                if not flag:
                    break
                offset += 25
            if not flag:
                break

        return self.time_table
                


if __name__ == "__main__":
    # Need to generate multiple times so that the rooms not crash, therefore creating no errors
    generator = TimetableGenerator()
    time_table1 = generator.generate_g_level("G1")
    time_table2 = generator.generate_g_level("G2")
    time_table3 = generator.generate_as_al_level()
    generator.add_pshe()

