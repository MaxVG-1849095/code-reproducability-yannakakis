select count(*) from imdb100, imdb123, imdb85 where imdb100.d = imdb123.d and imdb123.d = imdb85.s;