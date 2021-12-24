import bpy
import math
import json
import os

import logging

from .rafx_export_types import ExportContext

logging = logging.getLogger(__name__)

from . import rafx_blender_paths, rafx_errors, rafx_utils

def get_armature_data(armature: bpy.types.Armature):
    bone_data = []
    
    for bone in armature.bones:
        if bone.parent:
            parent_name = bone.parent.name
            parent_rotation = bone.parent.matrix
            # convert bone.head to space of parent bone
            offset = parent_rotation @ bone.head
            # combine
            offset = offset + (bone.parent.tail - bone.parent.head)
            # convert back to space of this bone
            offset = offset @ parent_rotation
            p = offset
        else:
            parent_name = ""
            p = bone.head
        
            
        r = bone.matrix.to_quaternion()
        s = bone.matrix.to_scale()
        bone_data.append({
            "name": bone.name,
            "parent": parent_name,
            "position": [p.x, p.y, p.z],
            "rotation": [r.x, r.y, r.z, r.w],
            "scale": [s.x, s.y, s.z]
        })
    
    return {
        "bones": bone_data
    }

def get_action_data(armature: bpy.types.Armature, action: bpy.types.Action, bone_default_transform_lookup):
    grouped_channels = {}
    for g in action.groups:
        for fc in g.channels:
            if not fc.data_path.startswith("pose.bones["):
                logging.warning("Keyframes found for unsupported data channel {}".format(fc.data_path))
                continue
            
            #TODO: Better way to find bone name and attribute name
            bone_name, attribute_name = fc.data_path.rsplit('.', 1)
            split_bone_name = bone_name.split("\"")
            if len(split_bone_name) < 2:
                logging.warning("Unsupported data path {}".format(fc.data_path))
                continue

            #TODO: Option to only export deform bones

            bone_name = split_bone_name[1]

            if attribute_name == "location":
                group_name = "position"
                grouped_index = fc.array_index
                grouped_channels.setdefault(bone_name, {}).setdefault(group_name, [None, None, None])[grouped_index] = fc
            elif attribute_name == "scale":
                group_name = "scale"
                grouped_index = fc.array_index
                grouped_channels.setdefault(bone_name, {}).setdefault(group_name, [None, None, None])[grouped_index] = fc
            elif attribute_name == "rotation_quaternion":
                group_name = "rotation"
                # swap w to be third component to match standard convention outside of blender
                grouped_index = [3, 0, 1, 2][fc.array_index]
                grouped_channels.setdefault(bone_name, {}).setdefault(group_name, [None, None, None, None])[grouped_index] = fc
            else:
                logging.warning("Keyframes found for unsupported data attribute {} on bone {}".format(attribute_name, bone_name))
                continue

    bone_channel_groups = []
    for bone_name in grouped_channels:
        bone_channel_group = {
            "bone_name": bone_name
        }

        for attribute in grouped_channels[bone_name]:
            min_frame = None
            max_frame = None

            channels = grouped_channels[bone_name][attribute]

            values = []

            default_data = bone_default_transform_lookup[bone_name][attribute]

            for i in range(0, len(channels)):
                fc = channels[i]
                if fc:
                    fc_range = fc.range()
                    range_begin = math.floor(fc_range[0])
                    range_end = math.ceil(fc_range[1])

                    if min_frame == None:
                        min_frame = range_begin
                        max_frame = range_end
                    else:
                        min_frame = min(min_frame, range_begin)
                        max_frame = max(max_frame, range_end)

            interpolation = [None] * (range_end - range_begin + 1)

            for i in range(0, len(channels)):
                fc = channels[i]
                if fc:
                    for kfp in fc.keyframe_points:
                        frame = round(kfp.co.x)
                        frame_index = frame - range_begin
                        if interpolation[frame_index]:
                            if kfp.interpolation != interpolation[frame_index]:
                                logging.warning("Keyframes with inconsistent interpolation setting at frame {} for bone {} attribute {}".format(frame, bone_name, attribute))
                        else:
                            interpolation[frame_index] = kfp.interpolation

            combined_interpolation = []
            previous_interpolation_mode = None
            for i in range(0, len(interpolation)):
                interpolation_mode = interpolation[i]
                if interpolation_mode and interpolation_mode != previous_interpolation_mode:
                    combined_interpolation.append({
                        "frame": i + range_begin,
                        "mode": interpolation_mode
                    })
                    previous_interpolation_mode = interpolation_mode

            
            for frame in range(range_begin, range_end + 1):
                value = default_data.copy()
                for i in range(0, len(channels)):
                    fc = channels[i]
                    if fc:
                        value[i] = fc.evaluate(frame)
                #TODO: Mark interpolation?
                values.append(value)

            bone_channel_group[attribute] = {
                "min_frame": min_frame,
                "max_frame": max_frame,
                "interpolation": combined_interpolation,
                "values": values
            }
            
            #bone_channel_data[bone_name]["min_frame"] = min_frame
            #bone_channel_data[bone_name]["max_frame"] = max_frame
            #bone_channel_data[bone_name]["values"] = values
            #bone_channel_data[bone_name]["interpolation"] = combined_interpolation
        bone_channel_groups.append(bone_channel_group)

    return {
        "name": action.name,
        "bone_channel_groups": bone_channel_groups
    }


def export_animation_data(export_context: ExportContext, obj: bpy.types.Object):
    if not export_context.visit_animation():
        return

    log_str = "Exporting animation data {}".format(object.name_full)
    export_context.info(log_str)
    logging.info(log_str)

    assert(object.type == "ARMATURE")
    
    actions = {}

    if obj.animation_data:
        action = obj.animation_data.action
        if action:
            k = (action.name, action.library)
            if k not in actions:
                actions[k] = action
    
    # For every track with exactly one non-muted strip, export the associated action
    for track in obj.animation_data.nla_tracks:
        non_muted_strips = [strip for strip in track.strips if strip.action and not strip.mute]
        if not track.strips or len(non_muted_strips) != 1:
            continue
        
        action = non_muted_strips[0].action
        k = (action.name, action.library)
        if k not in actions:
            actions[k] = action
    
    print("ARMATURE IS", obj.data)
    print("ACTION IS", actions)

    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(project_settings, obj)
    print(export_path)

    armature_data = get_armature_data(obj.data)

    bone_default_transform_lookup = {}
    for b in armature_data["bones"]:
        bone_default_transform_lookup[b["name"]] = {
            "position": b["position"],
            "rotation": b["rotation"],
            "scale": b["scale"]
        }

    all_actions = []
    for k in actions:
        all_actions.append(get_action_data(obj.data, actions[k], bone_default_transform_lookup))

    output_data = {
        "skeleton": armature_data,
        "actions": all_actions
    }
    armature_as_json = json.dumps(output_data, indent=4)
    #print(armature_as_json)
    rafx_utils.write_string_to_file(export_path, armature_as_json)
