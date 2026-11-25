# F8 prediction at 6744295f

- List 1 will contain at least one configured client call whose method and path have no schema operation.
- List 2 will contain at least one schema operation without an extracted registration for the same method and path suffix.
- List 3 will be the largest list. The Go extractor sees literal router registrations, including routes that a generated API schema may omit.
- List 4 will be smaller than list 3. I expect absent paths and out-of-schema routes to be more common than differing verbs for an otherwise related call or handler.

These predictions concern the extracted lists. R2 will determine the classifications and the G3 count.
