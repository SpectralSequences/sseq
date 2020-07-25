from setuptools import setup, Extension

with open("README.md", "r") as fh:
    long_description = fh.read()

setup(
    name="crappy-python-multitasking", # Replace with your own username
    version="0.1.0",
    author="Hood Chatham",
    author_email="roberthoodchatham@gmail.com",
    description="Crappy preemptive multitasking for Python in an environment without native multitasking.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/pypa/sampleproject",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    ext_modules=[Extension("crappy_multitasking", ["crappy_multitasking.c"])]
    # python_requires='>=3.6',
)
