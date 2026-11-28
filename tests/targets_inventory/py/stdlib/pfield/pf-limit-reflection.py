import builtins
from pathlib import Path


class ObjectSetattr:
    def __init__(self, path: Path):
        self.object_setattr = path

    def check_object_setattr(self):
        return self.object_setattr.exists()


def write_object_setattr(c, v):
    object.__setattr__(c, "object_setattr", v)


class InstanceSetattr:
    def __init__(self, path: Path):
        self.instance_setattr = path

    def check_instance_setattr(self):
        return self.instance_setattr.exists()


def write_instance_setattr(c, v):
    c.__setattr__("instance_setattr", v)


class BuiltinsSetattr:
    def __init__(self, path: Path):
        self.builtins_setattr = path

    def check_builtins_setattr(self):
        return self.builtins_setattr.exists()


def write_builtins_setattr(c, v):
    builtins.setattr(c, "builtins_setattr", v)


class DictUpdate:
    def __init__(self, path: Path):
        self.dict_update = path

    def check_dict_update(self):
        return self.dict_update.exists()


def write_dict_update(c, v):
    c.__dict__.update(dict_update=v)


class DictSetitem:
    def __init__(self, path: Path):
        self.dict_setitem = path

    def check_dict_setitem(self):
        return self.dict_setitem.exists()


def write_dict_setitem(c, v):
    c.__dict__.__setitem__("dict_setitem", v)


class GetattrDict:
    def __init__(self, path: Path):
        self.getattr_dict = path

    def check_getattr_dict(self):
        return self.getattr_dict.exists()


def write_getattr_dict(c, v):
    getattr(c, "__dict__")["getattr_dict"] = v


class DirectSetattr:
    def __init__(self, path: Path):
        self.direct_setattr = path

    def check_direct_setattr(self):
        return self.direct_setattr.exists()


def write_direct_setattr(c, v):
    setattr(c, "direct_setattr", v)
