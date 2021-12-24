import os
import json
import logging
import json

logging = logging.getLogger(__name__)

class ProjectSettings:
    art_dir: str
    assets_dir: str

ART_DIR = "art_dir"
ASSETS_DIR = "assets_dir"
PROJECT_ROOT_FILE_NAME = ".rafx_project"

def find_dir_containing_file_recursing_upwards(current_path, file_name):
    if not os.path.isabs(current_path):
        current_path = os.path.abspath(current_path)
    if os.path.isfile(current_path):
        dir_name = os.path.dirname(current_path)
        return find_dir_containing_file_recursing_upwards(dir_name, file_name)
    if os.path.isdir(current_path):
        if os.path.exists(os.path.join(current_path, file_name)):
            return current_path
        else:
            parent = os.path.dirname(current_path)
            if parent == current_path:
                return None
            return find_dir_containing_file_recursing_upwards(parent, file_name)

def sanitize_path(root_path, path):
    return os.path.join(root_path, path)

# current_path can be a file or directory, and can be absolute or relative
def find_project_settings(current_path) -> ProjectSettings:
    project_root = find_dir_containing_file_recursing_upwards(current_path, PROJECT_ROOT_FILE_NAME)
    if not project_root:
        return None

    project_file_path = os.path.join(project_root, PROJECT_ROOT_FILE_NAME)
    project_settings_json_obj = None
    with open(project_file_path, "r") as f:
        try:
            project_settings_json_obj = json.load(f)
        except json.decoder.JSONDecodeError:
            logging.error("Project file %s could not be parsed as json", project_file_path)
            return None

    project_settings = {
        ART_DIR: sanitize_path(project_root, project_settings_json_obj[ART_DIR]),
        ASSETS_DIR: sanitize_path(project_root, project_settings_json_obj[ASSETS_DIR])
    }

    logging.debug("Projects settings: %s", project_settings)
    
    return project_settings 
