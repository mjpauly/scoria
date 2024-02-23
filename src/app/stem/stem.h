// app initialization

typedef struct {
  uint16_t port;
  uint64_t scope;
} ServerConfig;

void set_app_dirs(
    const char* documents_dir,
    const char* library_dir,
    const char* temp_dir,
    const char* bundle_dir,
    const char* app_version
);

// log errors
void log_error(const char*);

// handle app state changes

void handle_shutdown(void);
void handle_enter_background(void);
ServerConfig handle_enter_foreground(void);

// sensor logging

typedef struct {
    // always available fields
    int64_t timestamp;
    double latitude;
    double longitude;
    double horizontal_accuracy;

    double msl_altitude;
    double ellipsoid_altitude;
    double vertical_accuracy;

    // bool indicates if story data is available
    bool story_available;
    int64_t story;

    // marked as unavailable with -1
    double speed;
    double speed_accuracy;
    double course;
    double course_accuracy;

    // indicates if source info is available, or should be NULL
    bool source_info_available;
    bool is_simulated_by_software;
    bool is_produced_by_accessory;
} OSLocationData;

void log_location(OSLocationData);

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

bool should_request_when_in_use_authorization(void);
bool should_export_sqlite_log(void);
bool should_import_sqlite_log(void);
void import_from_sqlite_log(const char*);
bool should_export_track(void);

bool get_bool(void);
