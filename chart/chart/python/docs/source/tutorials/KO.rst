KO HFPSS Tutorial
=================

Now we will make the HFPSS for KO. First we need to create our display object:

>>> chart = create_display("KO-HFPSS")

Let's set the range to be a bit larger: how about setting the x range to be from -20 to 20.
We do this with `chart.x_min`, `chart.x_max`. The attributes `chart.initial_x_min` and `chart.initial_x_max` determine
the size of the view of the chart when we first open it.

>>> chart.x_min = -20
... chart.x_max = 20
... chart.initial_x_min = -8
... chart.initial_x_max = 8
... chart.update()

Okay, now let's create the styles for the groups we will use. The only groups that appear are $\mathbb{Z}$, $2\cdot\mathbb{Z}$ and $\mathbb{Z}/2$.
We will use the standard circle for $\mathbb{Z}/2$, a solid black square `Shape` for $\mathbb{Z}$, and an outline-only square for $2\cdot\mathbb{Z}$.

>>> square = Shape.square(10)
... style = ChartClassStyle(group_name="Z", shape = square)

So ``style`` is now the style for our $\mathbb{Z}$ classes. We register this style with the chart using `chart.register_class_style`:

>>> chart.register_class_style(style)

This allows us to use "Z" as a short-hand for a solid black square.
Let's also make a "2Z" style with a transparent background by setting `ChartClassStyle.background_color`:

>>> style.group_name = "2Z"
... style.background_color = "transparent"
... chart.register_class_style(style)

Now let's add the classes:

>>> for v in range(-8, 9):
...     chart.add_class(4*v,0).set_style("Z") # Set the 0-line classes to be solid squares
...     for i in range(1, 20):
...         c = chart.add_class(4*v + i, i)
...         chart.add_structline((4*v+i-1, i-1), c)
... chart.update()

(Python ranges include the left endpoint and exclude the right endpoint, so for instance in the loop above $v\in \{-8, -7, \ldots, 8\}$.)

We want to give the classes descriptive names. These will appear in the tooltips. We use
`format_monomial` to automatically give tidy names in the special cases when exponents are 1 or 0.
Note also that for latex expressions it's important to use a "raw string" e.g., ``r"\alpha"``
See `the backslash plague <https://docs.python.org/3/howto/regex.html#the-backslash-plague>`_ for more info.

>>> for v in range(-8, 9):
...     for i in range(0, 20):
...         c = chart.get_class(4*v + i, i)
...         c.name = format_monomial(["v", v], [r"\eta", i])
... chart.update()

Now we want to add differentials. We use `ChartClass.replace` on the zero line sources to indicate
that twice these classes survive.

>>> for v in range(-7, 9, 2): # step from -7 to 7 by 2
...     for i in range(0, 17):
...         chart.add_differential(3, (4*v + i, i), (4*v + i - 1, i + 3))
...     c = chart.get_class(4*v, 0)
...     c.replace("2Z") # Use 2Z style starting on E_4 page
...     c.name[4] = rf"2\cdot {c.name[0]}"
... chart.update()

Lastly, we can clean up the page names. Rather than "page 2 with all differentials" we could just call it page 3.
We can see the list of pages in `chart.page_list`:

>>> chart.page_list
[(2, 65535), (65535, 65535)]

Let's replace the first pair with (3,3):

>>> chart.page_list[0] = (3,3)
... chart.update()

Finally we can save the chart:

>>> await chart.save_a()

A file picker dialog will open and you can choose where to save the chart. It can later be loaded with ``chart = await load_display_a("display_name")``.
