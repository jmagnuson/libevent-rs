#!/usr/bin/env python3

import matplotlib.pyplot as pyplot
import pandas


def main():
    df = pandas.concat([pandas.read_csv("target/release/kqueue.csv", names=["kqueue"]),
                        pandas.read_csv("target/release/tokio.csv", names=["tokio"])],
                        axis=1)

    df.plot()
    pyplot.title("Libevent Backend Benchmark (kqueue vs tokio)")
    pyplot.xlabel("Test Run")
    pyplot.ylabel("Time (µs)")
    pyplot.show()


if __name__ == "__main__":
    main()
