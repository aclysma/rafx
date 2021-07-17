
class RafxError(Exception):
    pass

class RafxFileNotSaved(RafxError):
    pass

class RafxFileNotWithinArtDir(RafxError):
    pass

class RafxProjectFileNotFound(RafxError):
    pass

class RafxNoExtensionForDataBlockType(RafxError):
    pass

class RafxSceneIsNotAModel(RafxError):
    pass

class RafxSceneIsNotAPrefab(RafxError):
    pass

class RafxModelSceneHasRootObject(RafxError):
    pass

class RafxPrefabSceneUnsupportedObject(RafxError):
    pass

class RafxModelSceneInvalidLodCollections(RafxError):
    pass

class RafxModelSceneHasNoCollections(RafxError):
    pass

class RafxModelSceneCollectionHasMultipleMeshes(RafxError):
    pass

class RafxModelSceneCollectionHasNoMeshes(RafxError):
    pass

class RafxUnsupportedMaterialGraphNode(RafxError):
    pass

class RafxCantCalculateTangents(RafxError):
    pass