# Version numbers follow semver semantics.
#
# The build version is the pre-release identifier. If the version name is "1.4"
# and the build version is "2", then that's the same as "1.4.0-beta.2". All
# build versions are assumed to be beta releases; we don't currently worry
# about alpha releases or release candidates. The build version preceeds the
# main version string ("1.4.0-beta.2" is a beta build for the "1.4.0" release.)
#
# Version bumps between iOS and Android should happen in tandem to keep things
# synced, even if a release is not created.

# Public version used in iOS.
IOS_VERSION_NAME = "1.4.0"

# Beta/build version used in iOS testing. Can add more number segments
# separated by dots to further distinguish build versions.
IOS_BUILD_VERSION = "19"


# Public semver version, only used for display. The patch version increments
# when android updates are not timed exactly with iOS, allowing slip for
# synchronizing on the major/minor version numbers.
ANDROID_VERSION_NAME = "1.4.0-beta.19" # set to "1.4.0" for public release

# Monotonic version code for use in Android. Bump on each release, since it is
# used to determine if a new update exists. Increases prevent downgrading.
ANDROID_VERSION_CODE = "3"
