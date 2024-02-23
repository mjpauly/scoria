package info.scoria;

public class OSLocationData {
    public long timestamp;
    public double latitude;
    public double longitude;
    public double horizontal_accuracy;

    public double msl_altitude;
    public double ellipsoid_altitude;
    public double vertical_accuracy;

    // bool indivates if story data is available
    public boolean story_available;
    public long story;

    // marked as unavailable with -1
    public double speed;
    public double speed_accuracy;
    public double course;
    public double course_accuracy;

    // indicates if source info is available, or should be NULL
    public boolean source_info_available;
    public boolean is_simulated_by_software;
    public boolean is_produced_by_accessory;
}
