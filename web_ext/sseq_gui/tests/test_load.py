import pytest


@pytest.mark.parametrize("module", ["S_2", "S_3", "C2v14"])
def test_load(driver, module: str):
    driver.go("/")
    driver.driver.find_element_by_css_selector(f'a[data="{module}"]').click()
    driver.wait_complete()
    driver.check_svg(f"{module}_load.svg")
