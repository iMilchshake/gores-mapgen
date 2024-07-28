import json
import sys
import os
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger(__name__)


def insert_at_position(d, key, value, position):
    items = list(d.items())
    items.insert(position, (key, value))
    return dict(items)


def convert_to_1_0(old_json: dict):
    inner_size_probs = old_json.get("inner_size_probs", [])
    outer_margin_probs = old_json.get("outer_margin_probs", [])
    shift_weights = old_json.get("shift_weights", [])

    # normalize
    shift_weights = [w / sum(shift_weights) for w in shift_weights]

    new_json = old_json.copy()
    new_json["inner_size_probs"] = {
        "values": [item[0] for item in inner_size_probs],
        "probs": [item[1] for item in inner_size_probs]
    }
    new_json["outer_margin_probs"] = {
        "values": [item[0] for item in outer_margin_probs],
        "probs": [item[1] for item in outer_margin_probs]
    }
    new_json["shift_weights"] = {
        "values": None,
        "probs": shift_weights
    }

    new_json = insert_at_position(new_json, "version", "1.0", 2)

    logger.info("> updated to version 1.0")
    return new_json


def main():
    if len(sys.argv) < 2:
        print("Usage: python converter.py <file1> <file2> ... <fileN>")
        sys.exit(1)

    for path in sys.argv[1:]:
        logger.info(f"loading {path}")
        with open(path, 'r') as file:
            config = json.load(file)

        # convert to v1.0
        new_config = convert_to_1_0(config)

        # backup old config
        backup_path = f"{path}.bak"
        os.rename(path, backup_path)

        # write new file to intial path
        with open(path, 'w') as file:
            json.dump(new_config, file, indent=2)

        logger.info(f"saved {path}")


if __name__ == "__main__":
    main()
