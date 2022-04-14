import time


def test_c3(driver):
    driver.go("/?module=C3&degree=36")
    driver.wait_complete()

    driver.click_class(18, 2)
    driver.send_keys("d")
    driver.click_class(17, 4)
    driver.reply("[1]")

    driver.click_class(19, 2)
    driver.send_keys("d")
    driver.click_class(18, 4)
    driver.reply("[2]")

    # Differential propagation checks that v₁ and β products are working
    driver.check_pages("C3_differential", 3)


def test_calpha(driver):
    driver.go("/?module=Calpha&degree=36")
    driver.wait_complete()

    driver.click_class(0, 0)
    driver.send_keys("p")

    driver.main_svg().click()
    driver.select_panel("Prod")

    driver.click_button("Add")
    driver.click_button("Show more")

    driver.reply("20")
    driver.wait_complete()

    driver.zoom_out("unit")
    driver.click_class(18, 2, False)
    driver.click_button("Add differential")
    driver.click_class(17, 4, False)
    driver.reply(True)
    time.sleep(0.1)
    driver.reply("[1]")
    time.sleep(0.1)
    driver.reply("[1]")
    time.sleep(0.1)
    driver.reply("g_2")
    time.sleep(0.1)
    driver.reply("g_1b")

    # Differential propagation checks that v₁ and β products are working
    driver.check_pages("Calpha_differential", 3)
