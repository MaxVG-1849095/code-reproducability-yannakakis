select count(*) from imdb2, imdb78, imdb72 where imdb2.d = imdb78.s and imdb78.s = imdb72.s;