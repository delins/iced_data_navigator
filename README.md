# Iced data navigator 

Widgets and virtual scrolling utilities to efficiently view and navigate large amounts of data with the Iced GUI library.

<div align="center">
<a href="https://github.com/delins/iced_data_navigator/blob/main/examples/hex_showcase/hex_showcase.gif">
  <img src="https://github.com/delins/iced_data_navigator/blob/main/docs/images/hex_showcase.png" width="600px"/>
</a>
</div>

## Overview
Currently contains:
- a hex viewer widget
- a horizontal and vertical scrollbar, and a wrapper that combines the two, that you can use as building blocks in your custom widget

Written for Iced's 0.14.0-dev branch.

## Hex viewer
The hex viewer can be used to view and analyse files of practically any size. It looks like a typical hex editor, but is read only. It currently supports features such as scrolling and highlighting, and is visually fairly customizable. Some of the features that are planned are:
- seeking
- ability to add keyboard shortcuts
- context menus

The hex showcase example can be run with:
```
cargo run --release --package hex_showcase
```

You can use this widget to add basic binary analysis functionality to your application, or create a fully fledged hex viewer application. It's currently not written with editing support in mind, but it probably doesn't need a lot of added features to allow it to be made into one without changing the widget code itself. I might look into it if there's any interest for it.

## Virtual scrolling utilities

When handling large amounts of data not everything can be loaded into your GUI at once: even if the data fits into your system's RAM your application would be terribly slow. 
One of the solutions is to use *virtual scrolling*: the application gives the impression that everything is available at once, but in reality it loads just the data it needs depending on the scrollbar's current position. 
Widgets that do virtual scrolling are often tailored to exact needs, but they typically perform the same type of scrolling. This crate offers reusable horizontal and vertical scrollbars as utility structs that can be used to build such widgets.
There is also a convenience struct that wraps both and adds wheel scrolling.

## Contributing

Pull requests and issues are welcome. But if you plan to invest much time into a pull request please get in touch first, since I'm still fleshing out the API and already have plans for additional features. You can reach me here or in the Iced Discord server, linked to on [Iced's github](https://github.com/iced-rs/iced).
