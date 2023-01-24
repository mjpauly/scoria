#import "RustLib.h"

// Rust function headers
// Rust funcs are in snake case whereas Objective C funcs are in camel case
extern int32_t get_a_value_from_rust(void);
extern int32_t log_location(double lat, double lon, double accuracy,
                            double speed, double course);
extern void write_file(const char* dir);
extern void thread_test(void);

NSInteger answer() {
    return get_a_value_from_rust();
}

int LogLocation(double lat, double lon, double accuracy, double speed,
                double course) {
    return log_location(lat, lon, accuracy, speed, course);
}

void TestWriteFileRust(const char* dir) {
    write_file(dir);
}

void TestThread(void) {
    thread_test();
}
