



- DefaultTextStyle is specified in Material widget
- Material widget selects localized Theme.of(context).textTheme.bodyMedium
- ThemeData::textTheme comes from Typography. However, ThemeData::textTheme::bodyMedium stores only color data. All size data is stored in ThemeData::textTheme::englighLike
- Theme.of(context) has additional effect of merging size data from `ThemeData::textTheme::englishLike` into `ThemeData::bodyMedium`
    - There are three sets of size data available: `englishLike` `dense` and `tall`
    - These three sets have different font sizes and font weights
    - However, in Material 3 (i.e. material2021, englishLike2021), these three sets are completely identical (except for name and baseline type)