#import <Foundation/Foundation.h>

void SetDocumentsDir(const char* dir);
int LogLocation(double lat, double lon, double accuracy, double speed,
                double course, long datetime_epoch);
void GenPastWeekViz(void);

// test examples
NSInteger answer();
void TestWriteFileRust(const char* dir);
