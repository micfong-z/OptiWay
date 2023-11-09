#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("optiway/include/json.hpp");
        include!("optiway/include/floyd.hpp");

        type json;
        type Graph;

        fn createSchoolGraph(path: &json;
    }
}
