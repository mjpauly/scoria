#import <Foundation/Foundation.h>

// app initialization

typedef struct {
  uint16_t port;
  uint64_t scope;
} ServerConfig;

void set_app_dirs(
    const char* documents_dir,
    const char* library_dir,
    const char* temp_dir,
    const char* bundle_dir
);

// handle app state changes

void handle_shutdown(void);
void handle_enter_background(void);
ServerConfig handle_enter_foreground(void);

// sensor logging

void log_location(double lat, double lon, double accuracy,
                     double speed, double course, long datetime_epoch);

// location settings

typedef enum {
    Best,
    TenMeters,
    HundredMeters,
    Kilometer,
    ThreeKilometers,
} LocAccuracyMode;

bool get_location_enabled(void);
float get_distance_filter(void);
bool get_significant_changes(void);
LocAccuracyMode get_location_accuracy_mode(void);
