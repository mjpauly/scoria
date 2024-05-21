# - Major version increments on backwards-incompatible changes, such as
# database migrations. Forwards-incompatible changes don't happen, so we don't
# use the major version number for that.
# 
# - Minor version increments for compatible changes.
#
# Version bumps between iOS and Android should happen in tandem to keep things
# synced, even if a release is not created.


# Public semver version use in iOS
IOS_VERSION_NAME = "1.4"

# Build version used in iOS testing. See comment in //src/app/ios/top/BUILD for
# more info.
IOS_BUILD_VERSION = "1.4.0"


# Public semver version, only used for display. The patch version increments
# when android updates are not timed exactly with iOS, allowing slip for
# synchronizing on the major/minor version numbers.
ANDROID_VERSION_NAME = "1.4.0"

# Monotonic version code for use in Android. Bump on each release, along with
# the patch version. Prevents downgrading, but we use semver to indicate when
# downgrades are possible with a database export/import.
ANDROID_VERSION_CODE = "3"
