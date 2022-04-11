from selenium.webdriver.common.by import By


def test_rotate_panel(driver):
    driver.go("/?module=tmf2")
    driver.wait_complete()

    assert driver.panel().find_element(By.TAG_NAME, "h2").text == "Vanishing line"

    # Now navigate to history panel
    driver.send_keys("J")
    assert len(driver.panel().find_elements(By.CSS_SELECTOR, "*")) == 0

    # Back to Main
    driver.send_keys("JJ")
    assert driver.panel().find_element(By.TAG_NAME, "h2").text == "Vanishing line"

    # Now to Prod panel
    driver.send_keys("K")
    assert len(driver.panel().find_elements(By.TAG_NAME, "details")) == 3


def test_structline_style(driver):
    # We should be there already but for clarity
    driver.select_panel("Prod")
    details = driver.panel().find_elements(By.TAG_NAME, "details")
    details[0].click()
    details[1].click()

    for row in details[0].find_elements(By.TAG_NAME, "input-row"):
        if row.get_attribute("label") == "Color":
            input_ = row.find_element(By.TAG_NAME, "input")
            input_.clear()
            input_.send_keys("red")

    for row in details[1].find_elements(By.TAG_NAME, "input-row"):
        if row.get_attribute("label") == "Bend":
            input_ = row.find_element(By.TAG_NAME, "input")
            input_.clear()
            input_.send_keys("20")

        if row.get_attribute("label") == "Dash":
            input_ = row.find_element(By.TAG_NAME, "input")
            input_.clear()
            input_.send_keys("0.05,0.05")

    driver.panel().find_elements(By.CSS_SELECTOR, "div > label > span.slider")[
        2
    ].click()

    driver.check_svg("tmf_structline_style.svg")
