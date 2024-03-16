# this is just meant to show that the most outwards positions have the longest distance, so: (0, x) or (0, y). this means that the maximum radius can be calculated as
def calculate_values(size):
    center = (size-1)/2

    # how 'deep' a corner can max be
    max_corner = int((size/2) - 1)
    print("max", max_corner)

    for corner_depth in range(max_corner+1):

        distances = list()
        for x in range(0, corner_depth+1):
            y = corner_depth - x
            dist = (center - x)**2 + (center - y)**2
            distances.append(dist)

        print(corner_depth, max(distances), distances)


def get_max_radii(size):
    radii = list()
    center = (size-1)/2

    max_corner = int((size/2) - 1)
    for corner_depth in range(max_corner+1):
        # x = 0, y = x - corner_depth = corner_depth
        dist = (center)**2 + (center - corner_depth)**2
        radii.append(dist)

    return radii


def get_dist(size, corner_depth):
    center = (size-1)/2
    return (center)**2 + (center - corner_depth)**2


def get_max_radii_simpl(size):
    return [get_dist(size, corn_depth) for corn_depth in range(size//2)]


def totally_cursed(size):
    return [(((size-1)/2)**2 + ((size-1)/2 - corn_depth)**2) for corn_depth in range(size//2)]


def wtf_is_going_on(s: float, d: int):
    return ((s-1)**2/2) - ((s-1) * d) + d**2


def even_more_cursed(size):
    return [wtf_is_going_on(size, corn_depth) for corn_depth in range(size//2)]


if __name__ == "__main__":
    for size in range(19):
        print(size, even_more_cursed(size))
