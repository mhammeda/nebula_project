<template>
    <div class="communityPostsArea">
        <v-btn
            small
            color="accent"
            @click="createPostOverlay = !createPostOverlay"
            class="ml-10 mb-3 text--primary"
        >
            <v-icon>
                mdi-note-plus
            </v-icon>
            Create a post
        </v-btn>
        <v-divider />
        <v-overlay :z-index="zIndex" :value="createPostOverlay">
            <CreatePost
                :communityID="communityId"
                v-on:postSuccesful="onPostSuccesful"
                v-on:close="createPostOverlay = false"
            />
        </v-overlay>

        <div class="listOfPosts">
            <v-menu open-on-hover top offset-y>
                <template v-slot:activator="{ on, attrs }">
                    <v-btn
                        color="secondary mx-10 mt-2"
                        dark
                        v-bind="attrs"
                        v-on="on"
                    >
                        Sort by
                    </v-btn>
                </template>

                <v-list>
                    <v-list-item
                        v-for="(item, index) in filterOptions"
                        :key="index"
                    >
                        <v-list-item-title v-on:click="item.associatedFunction"
                            >{{ item.title }}
                        </v-list-item-title>
                    </v-list-item>
                </v-list>
            </v-menu>
            <ul>
                <div v-for="item of communityPosts" v-bind:key="item.id">
                    <Post v-on:deletePost="getPosts" :item="item" />
                </div>
            </ul>
        </div>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </div>
</template>

<script>
import Post from '../posts/post.vue'
import CreatePost from '../posts/createPost.vue'

export default {
    mounted() {
        this.getPosts()
        console.log(this.communityId)
    },
    components: {
        Post,
        CreatePost,
    },
    data() {
        return {
            communityId: this.$route.params.communityID,
            postsUrl: '/internal/posts',
            communityPosts: [],
            createPostOverlay: false,
            zIndex: 10,

            filterOptions: [
                {
                    title: 'Date and time',
                    associatedFunction: () => {
                        this.sortPostsByTime()
                    },
                },
                {
                    title: 'Title',
                    associatedFunction: () => {
                        this.sortPostsByTitle()
                    },
                },
                {
                    title: 'Host',
                    associatedFunction: () => {
                        this.sortPostsByHost()
                    },
                },
                {
                    title: 'Username',
                    associatedFunction: () => {
                        this.sortPostsByUser()
                    },
                },
                {
                    title: 'Content',
                    associatedFunction: () => {
                        this.sortPostsByContent()
                    },
                },
            ],

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        getPosts() {
            this.errorMessage = ''
            this.$http
                .get(this.postsUrl, {}, { withCredentials: true })
                .then(response => {
                    const temp = []
                    if (response.data.length > 0) {
                        const allPosts = response.data
                        console.log('received ' + allPosts.length + ' posts')

                        let i
                        for (i = 0; i < allPosts.length; i++) {
                            if (allPosts[i].community == this.communityId) {
                                temp.push(allPosts[i])
                            }
                        }
                    } else {
                        this.errorMessage = 'No Posts to show :('
                        this.errorSnackbar = true
                    }
                    this.communityPosts = temp
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },

        onPostSuccesful(value) {
            // hides the allPostOverlay
            this.createPostOverlay = false

            this.getPosts()
        },

        sortPostsByTime() {
            this.communityPosts.sort((a, b) => {
                return b.created - a.created
            })
        },

        sortPostsByTitle() {
            this.communityPosts.sort((a, b) => {
                if (a.title > b.title) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByHost() {
            this.communityPosts.sort((a, b) => {
                if (a.author.host > b.author.host) {
                    if (a.author.username > a.author.username) {
                        return 1
                    } else {
                        return -1
                    }
                } else {
                    return -1
                }
            })
        },

        sortPostsByUser() {
            this.communityPosts.sort((a, b) => {
                if (a.username > b.username) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByContent() {
            this.communityPosts.sort((a, b) => {
                if (a.content[0].text.text > b.content[0].text.text) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByLikes() {
            /** To be changed and deployed
            this.communityPosts.sort((a,b) => {
                if (a.likes > b.likes) {
                    return 1
                } else {
                    return -1
                }
            })
            */
        },
    },
}
</script>
