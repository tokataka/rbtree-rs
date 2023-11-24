# RB-Tree for Rust

A sorted map implemented with RB-Tree[^1].

I made this to practice rust programming and documenting.

I followed method naming and example from Rust Standard Library [std::collections::BTreeMap](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html).

## Example

```rust
use rbtree::RbTree;
let mut movie_reviews = RbTree::new();

// review some movies.
movie_reviews.insert("Office Space", "Deals with real issues in the workplace.");
movie_reviews.insert("Pulp Fiction", "Masterpiece.");
movie_reviews.insert("The Godfather", "Very enjoyable.");
movie_reviews.insert("The Blues Brothers", "Eye lyked it a lot.");

// check for a specific one.
if !movie_reviews.contains_key("Les Misérables") {
    println!(
        "We've got {} reviews, but Les Misérables ain't one.",
        movie_reviews.len()
    );
}

// oops, this review has a lot of spelling mistakes, let's delete it.
movie_reviews.remove("The Blues Brothers");

// look up the values associated with some keys.
let to_find = ["Up!", "Office Space"];
for movie in &to_find {
    match movie_reviews.get(movie) {
        Some(review) => println!("{movie}: {review}"),
        None => println!("{movie} is unreviewed."),
    }
}

// Look up the value for a key (will panic if the key is not found).
println!("Movie review: {}", movie_reviews["Office Space"]);

// iterate over everything.
for (movie, review) in &movie_reviews {
    println!("{movie}: \"{review}\"");
}
```

[^1]: https://en.wikipedia.org/wiki/Red%E2%80%93black_tree

## License

This project is licensed under the terms of the MIT license.
