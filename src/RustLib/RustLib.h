#import <Foundation/Foundation.h>

void SetDocumentsDir(const char* dir);
int LogLocation(double lat, double lon, double accuracy, double speed,
                double course, long datetime_epoch);
void GenPastWeekViz(void);
void GenViz(long datetime_epoch_start, long datetime_epoch_end,
            double r, double g, double b, double a);

// test examples
NSInteger answer();
void TestWriteFileRust(const char* dir);
