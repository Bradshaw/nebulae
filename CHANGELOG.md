## 2.0.0 (2023-01-10)

### Feat

- pivoted to rayon, using atomic data types in raw_image to avoid mutexes
- clean up unecessary clear() and unused use statement
- clear terminal after tqdm finishes
- replace indicatif with tqdm

### Refactor

- remove superfluous use statement
- **jitter-sampler**: use thread-shareable rng
- **threading**: use mutex on raw_image to update the image in the threads instead of waiting for all threads to complete

## 1.0.0 (2022-10-18)

### Refactor

- **jitter-sampler**: remove unnecessary iteration counter
