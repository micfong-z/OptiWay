"""
Return File Structure (JSON):
    TimeTable: {
        student_number1 (string): {
            day (string): {
                period (string): room (string)
            }
            ...
        }
        ...
    }

Example:
    TimeTable: {
        "21447": {
            "1": {
                "1": "531",
                "2": "531",
                "3": "430",
                "4": "430"
                ...
            },
            "2": {...}
        },
        "21448": {
            "1": {
                "1": "210",
                "2": "210",
                "3": "322",
                "4": "322"
                ...
            },
            "2": {...}
        }
    }
"""

import random
import copy


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
            1.4 distribute 80 courses into PE, Bus, Hum, Enr, Lan, Sci following common-sense weightings (dict "elective_freq")
                1.4.1 Extra(art/drama/pe/music): 12
                1.4.2 Bus (econ/bus): 18
                1.4.3 Hum (geo+his+gp): 14
                1.4.4 Enr (cs): 6
                1.4.5 Lan (fren/span/ja): 5
                1.4.6 Sci (bio+phy): 25
            1.5 distribute timeslots for each course
                1.5.1 assign fixed scheduling
                    - SCIE has fixed scheduling - it separates classes into different timeslot couples
                       e.g, courses at Monday P1 would repeat at Wednesday P1
                    - 19 2-periods in a week, 18 non-PE, 2-periods in a week, 9 schedule sets,
                    - 8 clean schedule sets (without enrichment and another eng)
                    - schedule sets for core and electives are separated
                    - hence, distribute each class into these sets
                1.5.2 define schedule sets
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
            1.6 distribute the rooms for each course
                1.6.1 the rooms available for each floor is stored in the dict "avail_rooms"
                1.6.2 limitations: courses at each scheduling set should not have the same room
            1.7 distribute each student's list of courses taken
                1.7.1 distribute each student to a core class
            1.8 project back to generate the timetable

        :param: grade: either G1 or G2
        :return: timetable for the class
        """
        # Procedures #
        # 1.5 distribute timeslots for each course, each slot only needs 20 courses
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

        print("elective slots", elect_slots)

        # 1.6 distribute the rooms for each course
        if grade == "G1":
            slots = core_slots + elect_slots
        else:
            slots = elect_slots + core_slots
        rooms = {}
        filled_rooms = []  # track the filled rooms for each floor for each slot
        for index, slot in enumerate(slots):
            filled_room = dict(zip(range(2, 9), [set() for _ in range(2, 9)]))
            for course in slot:
                print(course)
                if course[:3] == "Sci":  # sci courses are distributed to three floors
                    floor = random.choice([6, 7, 8])  # TODO: change to greedy floor
                else:
                    floor = self.floor_map[course.split("_")[0]]
                    if floor == 1:  # if it is extra, just assign G
                        rooms[course] = "G"
                        continue
                # divide by the current filled rooms, use mod because the core slots for g1 are elective slots for g2
                room = random.choice(list(self.avail_rooms[floor] - filled_room[floor] - self.filled_rooms[(index + 4) % 8][floor]))
                if grade == "G2":
                    print(self.filled_rooms[(index + 4) % 8][floor])
                rooms[course] = self.map_building(room) + str(room)
                print(course, floor, room)
                filled_room[floor].add(room)
                print(index, filled_room)
            filled_rooms.append(filled_room)

        print(filled_rooms)

        # update the global filled_rooms
        for i in range(8):
            for j in range(2, 9):
                self.filled_rooms[i][j].update(filled_rooms[i][j])
                # [514, 519, 521, 523, 530, 531, 532, 533, 534, 502, 504, 508, 511]

        print(self.filled_rooms)

        print(rooms)
        # 1.7 distribute each student's list of courses taken
        if grade == "G1":
            offset = 22000
        else:
            offset = 21000
        students = dict(zip(range(offset+1, offset+501), [[] for _ in range(500)]))
        # 1.7.1 assign each student to a core class (#0 - 19)
        core_counts = [0 for _ in range(20)]  # track the number of students in each core class
        cores = list(range(20))
        for student in students:
            core = random.choice(cores)
            students[student].extend([f"Enr_core_{core}", f"Chi_core_{core}", f"Eng_core_{core}", f"Sci_core_{core}"])
            core_counts[core] += 1
            if core_counts[core] == 25:
                cores.remove(core)

        elect_slots_copy = copy.deepcopy(elect_slots)
        # 1.7.2 assign each student to an elective class
        filled_courses = {i: dict(zip(elect_slots_copy[i], [0] * 20)) for i in range(4)}  # track the filled courses for each slot
        for student in students:
            for i, slot in enumerate(elect_slots_copy):
                course = random.choice(slot)
                students[student].append(course)
                filled_courses[i][course] += 1
                if filled_courses[i][course] == 25:
                    elect_slots_copy[i].remove(course)

        print(students)
        print(elect_slots)
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
                        print(f"{k} {day} {p1} {p2}")
                        day, p1, p2 = self.glevel_set_map[core_set][index][1].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        print(f"{k} {day} {p1} {p2}")
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
                        print(f"{k} {day} {p1} {p2}")
                        day, p1, p2 = self.glevel_set_map[elect_set][index][1].split("_")
                        timetable[str(k)][day][p1] = rooms[course]
                        timetable[str(k)][day][p2] = rooms[course]
                        print(f"{k} {day} {p1} {p2}")

        self.filled_rooms[8][5].update(filled_rooms[8][5])

        print("filled", self.filled_rooms)

        return timetable


if __name__ == "__main__":
    generator = TimetableGenerator()
    time_table1 = generator.generate_g_level("G1")
    time_table2 = generator.generate_g_level("G2")
    print(time_table1)
    print(time_table2)
