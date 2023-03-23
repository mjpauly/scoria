#import <Foundation/Foundation.h>

uint16_t set_app_dirs(
    const char* documents_dir,
    const char* library_dir,
    const char* temp_dir,
    const char* bundle_dir
);
void log_location(double lat, double lon, double accuracy,
                     double speed, double course, long datetime_epoch);
float get_distance_filter(void);
