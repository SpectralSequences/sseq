import setuptools

with open("README.md", "r") as fh:
    long_description = fh.read()

setuptools.setup(
    name="spectralsequence_chart",
    version="0.0.4",
    author="Hood Chatham",
    author_email="roberthoodchatham@gmail.com",
    description="A Python implementation of the spectral sequences chart API",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/SpectralSequences/sseq",
    packages=setuptools.find_packages(),
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires='>=3.6',
)