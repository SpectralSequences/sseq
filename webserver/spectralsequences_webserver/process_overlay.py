# xmin = 0
# xmax = 5160 - 1920
# ymin = 185
# ymax = 2160 - 100

# xmin = 0 # Laptop monitor
# ymin = 185
# xmax = 3840
# ymax = 2060

xmin = 1243 # Ipad
ymin = 2160
xmax = 2609
ymax = 3184

from imageio import imread
from skimage import color, measure
import matplotlib.pyplot as plt

import time

from shapely.geometry import polygon, Point

# time1 = time.time()

def process_overlay(input_file, output_file):
    fimg = imread(input_file)
    gimg = color.colorconv.rgb2grey(fimg)
    gimg = gimg[ymin : ymax, xmin : xmax]
    
    contours = measure.find_contours(gimg, 0.8) # This one line pretty much does everything

    fig, ax = plt.subplots()
    plt.xlim([0, gimg.shape[1]])
    plt.ylim([0, gimg.shape[0]])
    plt.gca().invert_yaxis()
    ax.axis('off')
    fig.set_size_inches(gimg.shape[1]/100,gimg.shape[0]/100)
    contour_polys = []
    # Paint output
    for n, contour in enumerate(contours):
        inside = False
        for poly in contour_polys:
            # Check if this contour lies inside one of the previous ones.
            if poly.contains(Point(contour[0])):
                inside = True
                break
        if inside:
            countour_color = "w" # If so, paint it white
        else:
            countour_color = "b" # If not, paint it blue
            contour_polys.append(polygon.Polygon(contour))
        ax.fill(contour[:, 1], contour[:, 0], countour_color, linewidth=0.2)
    # Write to file
    plt.savefig(output_file, transparent=True)