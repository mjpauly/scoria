#import "RustLib.h"

// Rust function headers
// Rust funcs are in snake case whereas Objective C funcs are in camel case
extern void set_documents_dir(const char* dir);
extern int32_t log_location(double lat, double lon, double accuracy,
                            double speed, double course, long datetime_epoch);
extern void gen_past_week_viz(void);
extern void gen_viz(long datetime_epoch_start, long datetime_epoch_end,
                    double r, double g, double b, double a);

void SetDocumentsDir(const char* dir)
{
    set_documents_dir(dir);
}

int LogLocation(double lat, double lon, double accuracy, double speed,
                double course, long datetime_epoch)
{
    return log_location(lat, lon, accuracy, speed, course, datetime_epoch);
}

void GenPastWeekViz(void)
{
    gen_past_week_viz();
}

void GenViz(long datetime_epoch_start, long datetime_epoch_end,
            double r, double g, double b, double a)
{
    gen_viz(datetime_epoch_start, datetime_epoch_end, r, g, b, a);
}


// test examples

extern int32_t get_a_value_from_rust(void);
extern void write_file(const char* dir);

NSInteger answer() {
    return get_a_value_from_rust();
}

void TestWriteFileRust(const char* dir) {
    write_file(dir);
}
