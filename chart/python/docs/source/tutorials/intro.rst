Basic Introduction
==================

First note that you must use a chromium-based browser. Popular chromium-based browsers include Google Chrome, Chromium, Brave, and Microsoft Edge. 
It **will not** work in Firefox, Safari, Opera, etc.

Navigate to the site https://spectralsequences.com/repl/ and wait for the page to load.

First create a ``SseqDisplay``:

>>> disp = SseqDisplay("my_chart")
SseqDisplay(name="my_chart", url="https://spectralsequences.com/dist/charts/my_chart", chart=SseqChart("my_chart", classes=[0 classes], edges=[0 edges]))

If you press :kbd:`Control` and click on the link, it will open a new tab with the chart. You should see a blank chart.

..
    ADD IMAGES HERE

The display has an associated `SseqChart` object which holds the state of the spectral sequence chart. 
Most of the manipulations we do will be on the chart, so it's a good idea to store the chart into a variable:

>>> chart = disp.chart
SseqChart("my_chart", classes=[0 classes], edges=[0 edges])

We can add a `ChartClass` to the chart with `chart.add_class <SseqChart.add_class>`:

>>> c = chart.add_class(0, 0)
ChartClass(0, 0, 0)

The display is still empty. To update the display, you must run `chart.update() <SseqChart.update>`:

>>> chart.update() 

Now you should see the class we just added in the chart. 

..
    ADD IMAGES HERE

Let's set the fill color of the class to be blue. The fill color is stored in `c.background_color <ChartClass.background_color>`.

>>> c.background_color = "blue"
PageProperty([(0, Color("blue"))])

The color name must be chosen from a list of `CSS colors <https://www.w3schools.com/cssref/css_colors.asp>`_, though you may add your own with
`chart.register_color`.
`PageProperty` is a special wrapper that is used to allow changing the color based on the page.
We can set the color of the class to change to orange on page 3 with:

>>> c.background_color[3] = "orange"

In the repl when you've typed this line you should see an orange square to the left of ``"orange"`` and if you hover the cursor over ``"orange"``
an interactive color picker will show up.
After updating the chart again, you should see that pressing :kbd:`←` and :kbd:`→` causes the class to alternate between blue and orange.

..
    Add image here

The shape of the class is controlled by the field `c.shape <ChartClass.shape>`. 
Let's make a square `Shape` and set the class to be a square:

>>> square = Shape.square(10)
... chart.register_shape("square", square)
... c.shape = square
... chart.update()

Now the class should be a square.


Let's add another class and a `ChartStructline`.

>>> chart.add_class(1, 1)
... chart.add_structline(c, (1, 1))
... chart.update()

The source and target arguments to `chart.add_structline` can either be a reference to the class or a tuple indicating the location of the class.
It would be equivalent to say ``chart.add_structline((0,0), (1,1))`` or ``chart.add_structline(c, (1,1,0))``. 

Now let's add a differential:

>>> chart.add_class(2, 0)
... chart.add_differential(1, (2, 0), (1, 1))
ChartDifferential(ChartClass(2, 0, 0), ChartClass(1, 1, 0))
>>> chart.update()

At this point, the source and target of the differential have disappeared, but the differential itself is not visible. 
On the chart, you can change the page back and forth by pressing the arrow keys :kbd:`←` and :kbd:`→`.
However, you can see that only two states are currently possible: "Page 2 with all differentials" and "Page ∞". 
Neither of these show the $d_1$ differential we just added. We can add a page showing only differentials of length 1 by running:

>>> chart.add_page_range(1, 1)
... chart.update()

Now if you press :kbd:`←` enough times, you will see page 1. 
Alternatively, with `chart.add_page_range(1, INFINITY) <SseqChart.add_page_range>` we could add "Page 1 with all differentials".
Suppose the class in degree (2, 0) is actually a copy of $\mathbb{Z}$ and we want to indicate that the differential has a kernel.
We can do this as follows: let's go with the convention that we denote $\mathbb{Z}$ with a square. We can use the square from 
before as the shape of the class at (2,0):

>>> c = chart.get_class(2,0)
... c.shape = square
... chart.update()

Now we want to prevent the class from disappearing after the $d_1$. The class disappears because `c.max_page` is 1. 
It was set to 1 automatically when we added the differential. We can set that back to `INFINITY`.


>>> c.max_page = INFINITY
... chart.update()

Now we want set the background_color to be transparent starting on page 2:

>>> c.background_color[2] = Color.TRANSPARENT

If we add an extension, it will only show on the $E_{\infty}$ page:

>>> chart.add_class(2, 3)
... chart.add_extension((2, 0), (2,3))

You can save the chart with:

>>> await disp.save_a()

A save dialog will open and you can choose a location to save the file.