import time


def test_differential(driver):
    driver.go("/?module=S_2&degree=36")
    driver.wait_complete()

    driver.click_class(15, 1)
    driver.send_keys("d")
    driver.click_class(14, 3)
    driver.reply("[1]")

    driver.click_class(15, 2)
    driver.send_keys("d")
    driver.click_class(14, 5)
    driver.reply("[1]")

    driver.click_class(17, 4)
    driver.send_keys("d")
    driver.click_class(16, 6)
    driver.reply("[1]")

    driver.click_class(18, 4)
    driver.send_keys("d")
    driver.click_class(17, 6)
    driver.reply("[0, 1]")
    time.sleep(0.1)
    driver.reply("[1]")

    driver.check_pages("S_2_differential", 4)


def test_multiplication(driver):
    driver.click_class(0, 0)
    driver.send_keys("p")

    driver.click_class(8, 3)
    driver.send_keys("m")
    driver.reply("c_0")
    time.sleep(0.1)
    driver.reply(True)

    driver.click_class(9, 5)
    driver.send_keys("m")
    driver.reply("Ph_1")
    time.sleep(0.1)
    driver.reply(True)

    driver.click_class(14, 4)
    driver.send_keys("m")
    driver.reply("d_0")
    time.sleep(0.1)
    driver.reply(True)

    driver.click_class(20, 4)
    driver.send_keys("m")
    driver.reply("g")
    time.sleep(0.1)
    driver.reply(True)

    driver.check_pages("S_2_multiplication", 4)


def test_propagate_differential(driver):
    driver.click_class(17, 4)
    driver.select_panel("Diff")
    driver.panel().find_element_by_css_selector("div.panel-line").click()
    driver.reply("e_0")
    time.sleep(0.1)
    driver.reply("h_1^2 d_0")

    driver.click_class(18, 4)
    driver.panel().find_elements_by_css_selector("div.panel-line")[3].click()
    driver.reply("f_0")
    time.sleep(0.1)
    driver.reply("h_0^2 e_0")

    driver.check_pages("S_2_propagate_diff", 4)


def test_undo_redo(driver):
    driver.click_button("Undo")
    driver.click_button("Undo")
    driver.check_pages("S_2_multiplication", 4)

    driver.click_button("Redo")
    driver.click_button("Redo")
    driver.check_pages("S_2_propagate_diff", 4)


def test_history(driver):
    driver.click_button("Save")
    driver.reply("s_2.save")
    time.sleep(0.1)

    with open(f"{driver.tempdir}/s_2.save") as f:
        driver.check_file("s_2.save", f.read())

    driver.go("/")
    driver.driver.find_element_by_id("history-upload").send_keys(
        f"{driver.tempdir}/s_2.save"
    )
    driver.wait_complete()

    driver.check_pages("S_2_propagate_diff", 4)
