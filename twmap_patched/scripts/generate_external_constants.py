import argparse
import pathlib
from PIL import Image
from os import path
import numpy as np

images_06 = [
    "bg_cloud1",
    "bg_cloud2",
    "bg_cloud3",
    "desert_doodads",
    "desert_main",
    "desert_mountains",
    "desert_mountains2",
    "desert_sun",
    "generic_deathtiles",
    "generic_unhookable",
    "grass_doodads",
    "grass_main",
    "jungle_background",
    "jungle_deathtiles",
    "jungle_doodads",
    "jungle_main",
    "jungle_midground",
    "jungle_unhookables",
    "moon",
    "mountains",
    "snow",
    "stars",
    "sun",
    "winter_doodads",
    "winter_main",
    "winter_mountains",
    "winter_mountains2",
    "winter_mountains3"
]

images_07 = [
    "bg_cloud1",
    "bg_cloud2",
    "bg_cloud3",
    "desert_doodads",
    "desert_main",
    "desert_mountains",
    "desert_mountains2",
    "desert_sun",
    "easter",
    "generic_deathtiles",
    "generic_lamps",
    "generic_shadows",
    "generic_unhookable",
    "grass_doodads",
    "grass_main",
    "jungle_background",
    "jungle_deathtiles",
    "jungle_doodads",
    "jungle_main",
    "jungle_midground",
    "jungle_unhookables",
    "light",
    "moon",
    "mountains",
    "snow",
    "stars",
    "sun",
    "winter_doodads",
    "winter_main",
    "winter_mountains",
    "winter_mountains2",
    "winter_mountains3",
]


if __name__ == "__main__":
    arg_parser = argparse.ArgumentParser(description="script to generate the external constants in src/constants/")
    arg_parser.add_argument("mapres_version", help="mapres version, '6' for 0.6, '7' for 0.7", choices=["6", "7"])
    arg_parser.add_argument("image_directory", help="directory where the external images can be read from", type=pathlib.Path)
    arg_parser.add_argument("output_file", type=pathlib.Path)
    args = arg_parser.parse_args()

    if args.mapres_version == "6":
        image_names = images_06
    elif args.mapres_version == "7":
        image_names = images_07

    opaque_tables = []
    image_dimensions = []
    for name in image_names:
        file_path = path.join(args.image_directory, name + ".png")
        with Image.open(file_path) as image:
            if image.format != "PNG":
                print(f"{file_path} is not a png image")
            if len(image.getbands()) != 4:
                print(f"{file_path} is not a rgba image")
            width, height = image.size
            image_dimensions.append((width, height))
            tile_width, tile_height = width // 16, height // 16
            if width % 16 != 0 or height % 16 != 0 or tile_width != tile_height:
                opaque_tables.append(None)
                continue
            pixels = np.frombuffer(image.tobytes()[3::4], dtype=np.uint8)
            alpha_values = pixels.reshape((height, width))
            opaque_table = [[False for i in range(16)] for j in range(16)]
            for y in range(16):
                for x in range(16):
                    opaque_table[y][x] = np.all(alpha_values[y * tile_height:(y + 1) * tile_height, x * tile_width:(x + 1) * tile_width] >= 250)
            opaque_tables.append(opaque_table)

    const_image_names = [name.upper() for name in image_names]

    with open(args.output_file, "w") as file:
        file.write("use vek::Extent2;\n\n")
        file.write("/// Returns `true` if all clients of that version should have that mapres shipped with their client.\n")
        file.write("pub fn is_external_name(name: &str) -> bool {\n")
        file.write("    matches!(\n")
        file.write("        name,\n")
        file.write("        \"")
        file.write("\"\n            | \"".join(image_names))
        file.write("\"\n    )\n")
        file.write("}\n\n")

        file.write("/// Returns the opaque table for external images.\n")
        file.write("/// None, if the given name is not a valid external image name, or if the image not square.\n")
        file.write("/// The latter restriction is taken from the client source, don't ask me.\n")
        file.write("pub fn external_opaque_table(name: &str) -> Option<[[bool; 16]; 16]> {\n")
        file.write("    Some(match name {\n")
        for name, const_name, table in zip(image_names, const_image_names, opaque_tables):
            if table is None:
                continue
            file.write(f"        \"{name}\" => {const_name},\n")
        file.write("        _ => return None,\n")
        file.write("    })\n")
        file.write("}\n\n")

        file.write("/// Returns the (width, height) of external images.\n")
        file.write("/// None, if the given name is not a valid external image name.\n")
        file.write("pub fn external_dimensions(name: &str) -> Option<Extent2<u32>> {\n")
        file.write("    Some(match name {\n")
        for name, (width, height) in zip(image_names, image_dimensions):
            file.write(f"        \"{name}\" => Extent2::new({width}, {height}),\n")
        file.write("        _ => return None,\n")
        file.write("    })\n")
        file.write("}\n")

        for const_name, opaque_table in zip(const_image_names, opaque_tables):
            if opaque_table is None:
                continue
            file.write("\n#[rustfmt::skip]\n")
            file.write(f"const {const_name}: [[bool; 16]; 16] = [\n")
            for line in opaque_table:
                array = [" true" if boolean else "false" for boolean in line]
                array = ", ".join(array)
                file.write(f"    [{array}],\n")
            file.write("];\n")
