select count(*) from imdb2, imdb123, imdb100, imdb86, imdb78 where imdb2.d = imdb123.d and imdb123.d = imdb100.d and imdb100.d = imdb86.s and imdb86.s = imdb78.s;