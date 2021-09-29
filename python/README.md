
# sudachi.rs python

This is the python binding of sudachi.rs.


# Caution

This project is under development and specifications may change drastically.


# Setup

1. Install python module `setuptools` and `setuptools-rust`.
2. Run `python3 setup.py develop`.
    - `develop` will create a debug build, while `install` will create a release build.
3. Now you can import the module by `import sudachi.sudachi`.

ref: [setuptools-rust](https://github.com/PyO3/setuptools-rust)


# Example

```python
import sudachi.sudachi as ss

dictionary = ss.Dictionary()
tokenizer = dictionary.create()
morphemes = tokenizer.tokenize("国会議事堂前駅")
print(morphemes[0].surface())  # '国会議事堂前駅'
print(morphemes[0].reading_form())  # 'コッカイギジドウマエエキ'
print(morphemes[0].part_of_speech())  # ['名詞', '固有名詞', '一般', '*', '*', '*']

tokenizer = dictionary.create(ss.SplitMode.A)
morphemes = tokenizer.tokenize("国会議事堂前駅")
print(list(map(str, morphemes)))  # ['国会', '議事', '堂', '前', '駅']
```
