from pathlib import Path


def extract_feature_flags(config_path: Path):
    content = config_path.read_text()
    return content
