#import <Foundation/Foundation.h>

void set_app_dirs(
    const char* documents_dir,
    const char* library_dir,
    const char* temp_dir,
    const char* bundle_dir
);
int32_t log_location(double lat, double lon, double accuracy,
                     double speed, double course, long datetime_epoch);
void gen_past_week_viz(void);
void gen_viz(long datetime_epoch_start, long datetime_epoch_end,
                    double r, double g, double b, double a);
