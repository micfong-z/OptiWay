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

## Path Evaluation

For each route $r$ for a student between two specific periods, we give the route a performance index $r_\mathrm{perf}$, where smaller performance index indicates better performance. Consider all the paths between nodes $p_i\ (i\in\{1, 2, 3, \dots, n\})$ in the route $r$, let the length of the path be $w_i$, and the number of students passing the path between the two periods (congestion) be $c_i$.

The performance index for $r$ is:

$$
\begin{align}
r_\mathrm{perf}&=\sum^n_{i=1}\left[ w_i\cdot\left(2+\frac{e^{(c_i-300)/200}-e^{-(c_i-300)/200}}{e^{(c_i-300)/200}+e^{-(c_i-300)/200}}\right)\right]\\
&=\sum^n_{i=1}\left[ w_i\cdot\left(2+\tanh\left(\frac{c_i-300}{200}\right)\right)\right]
\end{align}
$$

## Optimization Output

During path optimization, the program should continuously output lines in the following format for each batch:

```
[flag (0/1)] [iteration] [day# (1-5)] [period# (0-11)] [current r_perf] [previous best r_perf]
```

Where `[flag]` indicates whether a new route JSON file has been dumped after current iteration.

Example output:
```
1 12300 2 4 12345678 12345690
0 12400 2 5 12345678 12345678
0 12500 2 6 12345660 12345678
```

Note that the program should **not** output a line at `iteration=0`.

## Optimization CLI arguments

- `-f [path]` The source JSON file path.
- `-b [number]` The size of each batch.
- `-i [number]` The number of iterations each time.
- `-d [number]` The day to evaluate on.
