'''Takes a dictionary of targets containing individual files mapped to
environment variables to expose to the compiler, and returns a list of the
targets and a mapping of the environment variables to the target locations.
'''
def files_and_envs(names_envs):
    static_file_targets = names_envs.keys()
    static_file_env_defs = {
        envar: "$(location %s)" % target # map ENV_VAR -> location of target
        for target, envar in names_envs.items()
    }
    return static_file_targets, static_file_env_defs

