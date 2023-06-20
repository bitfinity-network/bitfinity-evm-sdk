# Issue ticket

Issue ticket link: <>

## Checklist before requesting a review

### Code conventions

- [ ] I have performed a self-review of my code
- [ ] Every new function is documented
- [ ] Object names are auto explicative

### Security

- [ ] The PR does not break APIs backward compatibility
- [ ] The PR does not break the stable storage backward compatibility

### Testing

- [ ] Every function is properly unit tested
- [ ] I have added integration tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] IC endpoints are always tested through the `canister_call!` macro
