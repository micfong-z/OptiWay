#[cxx::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("optiway/include/json.hpp");
        include!("optiway/include/utilities.hpp");
        include!("optiway/include/floyd.hpp");
        type Json;
        type Graph;
        fn createSchoolGraph(path: &str) -> UniquePtr<Graph>;
        fn getRoutesFromTimetable(
            timetables: &Json,
            graph: &Graph,
            shortest_paths: &Json,
        ) -> UniquePtr<Json>;

        fn makeJson(contents: &str) -> UniquePtr<Json>;
        fn jsonToString(json: &Json) -> UniquePtr<CxxString>;
    }
}

use std::{fs::File, io::BufReader};

use bridge::*;

// Translation of the `main` function in `floyd.cpp`
pub fn interop_with_badlang(filename: &str) {
    let graph = createSchoolGraph("../assets/paths.txt");
    let timetables = std::fs::read_to_string(filename).unwrap();
    let timetables = makeJson(&timetables);
    let paths_file = std::fs::read_to_string("../assets/paths.txt").unwrap();
    let shortest_paths = makeJson(&paths_file);
    let routes = getRoutesFromTimetable(&timetables, &graph, &shortest_paths);
    dbg!(jsonToString(&routes));
}
