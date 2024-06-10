



1. Intrinsics would also couple parent layout to children even if parent does not rely on children size
    1. This is achieved by overriding `RenderBox::markNeedsLayout` and pre-emptively marking their parent when any cached intrinsics are detected. (Such dirty hacks)
2. What's all the fuss about `RenderBox::computeDryLayout`?
    1. Flutter's original intrinsics fails drastically in certain cases.
        1. https://github.com/flutter/flutter/issues/48679, where the `Row` widget and intrinsics protocol just can't co-exist anymore, and `Row` will always lays out a different size than what it can possibly express under Flutter's intrinsics system.
        2. https://github.com/flutter/flutter/issues/65895, where the system is simply broken beyond repair and have to forbid the usage.
        3. Therefore, the Flutter team has no other option except to do a real layout for some inline spans inside text.
    2. Flutter's cost for actually laying out text seems too high and thus it must separate out two flavors of layout
        1. One is normal layout that computes out all text layout details, and the another one that only spit out text size.
    3. Therefore, we end up having normal layout and a dry layout that extract only sizes.
        1. Notably, dry layout most often will still visit its children and would just mimic its normal layout version. The only distinction is that when they fully resolve to the lowest text widgets, they don't try to fully layout the text and are happy with only the overall text size.
        2. In fact, Flutter render objects often writes their layout impl in such a way that they are polymorphic over function pointers to be generic over dry and wet layout.
    4. Conspicuously, `performResize` is stricly a stricter and narrower version of `computeDryLayout` when `sizedByParent == true`. Therefore after https://github.com/flutter/flutter/pull/70656, `performResize` is delegated to the implementation of `computeDryLayout`
3. How should we handle dry layout and the motiviation behind it?
    1. If our text layout is also expensive:
        1. Then expensive part must be a leaf widget (which means a black box / out of our control, thus expensive)
        2. Leaf widget can always be made `sizedByParent` in some sense.
        3. So we can still use the dry layout analogy used by Flutter. Provide a dry layout variant for leaf widget