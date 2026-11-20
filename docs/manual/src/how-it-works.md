# Architecture and strategy

The plugin is an effect. Each band is a biquad filter from Robert Bristow-Johnson's audio EQ cookbook. The code is ours, with no other library, and it runs on the audio thread with no latency and no memory allocated.

The strategy is to keep it simple and predictable: one slider per band, no hidden processing. The unit tests compare the filters against the cookbook's responses, and a real-time check confirms nothing allocates while playing.
