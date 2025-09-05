import sys
from pathlib import Path
from mkdocs.config.defaults import MkDocsConfig
from mkdocs.plugins import get_plugin_logger

logger = get_plugin_logger(__name__)

DOCS_PATH_ADDED = False

EXTENSIONS = {
    "extensions.collapse-code": {
        "expand_text": "",
        "collapse_text": ""
    }
}

def on_config(config: MkDocsConfig) -> MkDocsConfig:
    global DOCS_PATH_ADDED

    if not DOCS_PATH_ADDED:
        docs_folder = str(Path(config.config_file_path).parent.joinpath("docs").absolute())
        logger.info("adding docs folder to pythonpath: %s", docs_folder)
        sys.path.insert(0, docs_folder)
        DOCS_PATH_ADDED = True

    # Must be done here, as MkDocs tries to load the extensions when the config is parsed
    # So unless PYTHONPATH is set before running `mkdocs`, the config will fail to validate
    for ext, cfg in EXTENSIONS.items():
        # locally, mkdocs-static-i18n rebuilds the config multiple times, and this hook will 
        # be called even though the extension was already added. So this check is necessary
        if ext in config.markdown_extensions:
            continue

        logger.info("loading local markdown extension %s", ext)
        config.markdown_extensions.append(ext)
        config.mdx_configs[ext] = cfg

    return config