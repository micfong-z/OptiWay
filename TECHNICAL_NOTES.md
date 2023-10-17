# Technical Notes for OptiWay

This document records technical notes for future reference.

## Source Data

### Projection Coordinates `projection-coords.yaml`

Records the 3D projection coordinates of each node, which is used to render paths on the projection. See [3D-2D Projection Coordinates](#3d-2d-projection-coordinates) for convertion information.

The first-level key is the floor: `2F`, `3F`, `4F` etc.

The second-level key is one from `A`, `B` and `X`, where the first two of them represent the teaching block, and `X` represents "crossings" between two blocks.

The third-level key is the node with its coordinates in the format `[x, y, z]`, where `+x` is the east, `+y` is the north, and `+z` is vertically upwards. The node naming conventions are listed as follows:
- `X`: Crossings between two blocks.
 
    `XNKM`: The M-th crossing node on K block, N-th floor. The numbering order is from west to east.
- `A`: Nodes in the A block.
- `B`: Nodes in the B block.
- `S`: Stairs.

  SN-M: The M-th stair node on N-th floor.

### Path Information `paths.yaml`

Records the paths used to construct a undirected graph for route calculation.

The first-level key is the floor: `2F`, `3F`, `4F` etc., and a special key `S`, which denotes "stairs".

The second-level key is one from `A`, `B` and `X`, where the first two of them represent the teaching block, and `X` represents "crossings" between two blocks.

The thrid-level is a list of maps, where `dist` denotes the distance between two notes (arbitary unit). `nodes` denote the two nodes connected, and see [Projection Coordinates](#projection-coordinates-projection-coordsyaml) for naming conventions. `type` denote the path type, which is an integer of the followings:

- 0: Normal path between two rooms / between a room and one end of a bridge
- 1: Bridge between two buildings
- 2: Normal staircases (e.g. S2-3)
- 3: Spiral staircases between two floors only (e.g. S2-2)
- 4: Other types of staircases (e.g. S4-6)

### Projection Length `projection-length.yaml`

This is an intermediate file that needs no attention.

## User Interface

### 3D-2D Projection Coordinates

The coordinates recorded in `projection-coords.yaml` is converted to on-screen points using the following formula:

$$
f\left( \begin{bmatrix}
x \\
y \\
z
\end{bmatrix} \right) = \begin{bmatrix}
(25+x+y)\cos \frac{\pi}{6} \\
400 + (25+x-y)\sin \frac{\pi}{6}-z
\end{bmatrix}\cdot s+\boldsymbol{a}
$$

where $x, y, z$ are coordinates in the YAML file, $s$ is the scale factor of the image, and $\boldsymbol{a}$ is the offset from the top left point on screen.

The angle used is $\frac{\pi}{6}$ because the projection angle used is exactly $30°$.

## Timetable Generation

Timetables are generated following the conventions of how SCIE designs the timetables. The generations for G Level and AS & A Level students are separated as they have different timetable structures, i.e., the latter has spare classes.

For each grade level, the following steps are taken:
1. Calculate the total number of courses required for every student to take the courses.
2. Distribute the courses into different subjects based on defined frequencies.
3. Distribute the timeslots for each course.
4. Distribute the rooms for each course.
5. Distribute each student's list of courses taken
6. Project all data back to the desired timetable's data structure, as stored in `timetable_generation/return_structure.txt`

Detailed validation and calculation steps are recorded by comments in `timetable_generator.py`. We assume that the room for each course does not change throughout the week. In addition, all AS students take only 5 courses, and all AL students take 4 courses, while each AS and AL student is assigned a G&T class.


## Path Evaluation

For each route $r$ for a student between two specific periods, we give the route a performance index $r_\mathrm{perf}$, where smaller performance index indicates better performance. Consider all the paths between nodes $p_i\ (i\in\{1, 2, 3, \dots, n\})$ in the route $r$, let the length of the path be $w_i$, and the number of students passing the path between the two periods (congestion) be $c_i$.

The performance index for $r$ is:

$$
\begin{align}
r_\mathrm{perf}&=\sum^n_{i=1}\left[ w_i\cdot\left(2+\frac{e^{(c_i-300)/200}-e^{-(c_i-300)/200}}{e^{(c_i-300)/200}+e^{-(c_i-300)/200}}\right)\right]\\
&=\sum^n_{i=1}\left[ w_i\cdot\left(2+\tanh\left(\frac{c_i-300}{200}\right)\right)\right]
\end{align}
$$

## Floyd-warshall's Algorithm

Floyd-warshall's algorithm is utilized to generate the shortest paths from each room to another. These paths act as baselines for students' paths, as they do not take into account congestion. We pre-calculate the shortest path of every room pair and save them into a JSON file. At runtime, only data retrieval is required, improving the efficiency of our approach.


## Optimization Algorithm

We have developed a customized algorithm for optimizing the shortest paths absed on multiple objectives, i.e., congestion and distance, based on the performance index defined in [**Path Evaluation**](#path-evaluation). 

The algorithm works as below:

 **Initializations**
 
1. Calculate congestion $c_i$ for each edge
2. Assign precalculated shortest path to each student
3. Calculate the performance $r_\text{perf}$ for each student's path, and sum to get an initial total $\sum r_\text{perf}$
4. Store each student, their relative path, and the path's $r_\text{perf}$ score as a struct, and push the struct into a priority queue ordered by $r_\text{perf}$ decreasingly.

**Heurestic Iterations**

1. **For each iteration**, choose the student whose path has the greatest $r_\text{perf}$ 
2.  Recalculate the path of the student using **Dijkstra's algorithm**, by including a **congestion weight** $w_c$
    - for each edge $e_i \in E$, we update it as $w_{e_i} := w_{e_i} + w_c \times c_i  $
    - Calculate an individual $r_\text{perf}$ value for the updated path
      - note that $c_i$ is not updated here for efficiency, as a minor change in $c_i$ has minor changes to $\sum r_\text{perf}$
3. **For each batch_size iterations**, we recalculated $c_i$ at each edge, and deduce a new $\sum r_\text{perf}$. If the new $\sum r_\text{perf}$ is greater than that of the last batch, we update the paths.

The full implementation is studed in `multi_agent_path_finding/multi-objective-agent.cpp`. Pratically, we set the number of iterations to a very large number, letting the user to stop it manually.
