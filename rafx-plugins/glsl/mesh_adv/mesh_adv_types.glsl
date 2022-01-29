
// @[export]
struct Transform {
    mat4 model_matrix;
};

// @[export]
struct TransformWithHistory {
    mat4 current_model_matrix;
    mat4 previous_model_matrix;
};

// @[export]
struct DrawData {
    uint transform_index;
    uint material_index;
};
