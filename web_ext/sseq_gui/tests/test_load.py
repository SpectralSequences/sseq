import pytest
from pathlib import Path

from selenium.webdriver.common.by import By


@pytest.mark.parametrize("module", ["S_2", "S_3", "C2v14"])
def test_load(driver, module: str):
    driver.go("/")
    driver.driver.find_element(By.CSS_SELECTOR, f'a[data="{module}"]').click()
    driver.wait_complete()
    driver.check_svg(f"{module}_load.svg")


@pytest.mark.parametrize("module", ["S_2", "S_3", "C2v14"])
def test_load_json(driver, module: str):
    path = Path(__file__).parent / "../../../ext/steenrod_modules" / f"{module}.json"
    path = path.resolve()

    driver.go("/")
    driver.driver.find_element(By.ID, "json-upload").send_keys(str(path))
    driver.reply("40")
    driver.wait_complete()
    driver.check_svg(f"{module}_load.svg")
