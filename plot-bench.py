#!/usr/bin/env python3

import matplotlib.pyplot as pyplot
import pandas


def main():
    df = pandas.concat([pandas.read_csv("target/release/kqueue.csv", names=["kqueue"]),
                        pandas.read_csv("target/release/tokio.csv", names=["tokio"])],
                       axis=1)

    kqueue_mean = df["kqueue"].mean()
    tokio_mean = df["tokio"].mean()

    print(
        f"kqueue mean: {round(kqueue_mean)}, tokio mean: {round(tokio_mean)}, ratio: {round(100 * kqueue_mean / tokio_mean)}%")

    df.plot()
    pyplot.title("Libevent Backend Benchmark (kqueue vs tokio)")
    pyplot.xlabel("Test Run")
    pyplot.ylabel("Time (µs)")
    pyplot.show()


if __name__ == "__main__":
    main()
