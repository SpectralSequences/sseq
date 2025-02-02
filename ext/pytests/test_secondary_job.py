import subprocess


def folder_pairs(folder):
    filename_parts = (f.stem.split("_") for f in folder.glob("**/*") if f.is_file())
    return (
        (int(split[0].replace("m", "-")), int(split[1])) for split in filename_parts
    )


def test_secondary_job(tmp_path, monkeypatch):
    max_stems = 10
    goal_stem = 3
    monkeypatch.setenv("SECONDARY_JOB", str(goal_stem))
    subprocess.run(
        [
            "cargo",
            "run",
            "--example",
            "secondary",
            "--",
            "S_2",
            tmp_path,
            str(max_stems),
            "6",
        ],
        check=True,
    )
    for folder in [
        "augmentation_qis",
        "differentials",
        "kernels",
        "req_qis",
        "secondary_composites",
        "secondary_intermediates",
    ]:
        for n, _ in folder_pairs(tmp_path / folder):
            assert n <= max_stems

    for _, s in folder_pairs(tmp_path / "secondary_composites"):
        assert s == goal_stem

    for _, s in folder_pairs(tmp_path / "secondary_intermediates"):
        assert s == goal_stem + 1
