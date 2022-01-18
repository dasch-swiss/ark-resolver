"""Creates a pickle file 'settings.pkl' with the configuration in ark-registry.ini. This file can then be used in the
unit tests. Run it from inside ark-resolver with 'python3 test/create_mock_settings.py' """

import pickle as pkl
import os
import sys

sys.path.append('src')

from ark import load_settings

if __name__ == "__main__":
    config_path = "src/ark-config.ini"
    os.environ['ARK_REGISTRY'] = 'src/ark-registry.ini'
    settings = load_settings(config_path)
    with open("test/settings.pkl", 'wb') as file:
        pkl.dump(settings, file)
